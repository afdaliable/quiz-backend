use crate::model::soal::CreateSoalRequest;
use actix_multipart::Multipart;
use actix_web::{web, HttpResponse};
use csv::ReaderBuilder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use utoipa::ToSchema;

/// Configuration for CSV import service
pub struct CsvImportConfig {
    pub max_file_size: usize,
    pub max_questions_per_import: usize,
    pub allowed_delimiters: Vec<char>,
}

impl Default for CsvImportConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_questions_per_import: 10000,
            allowed_delimiters: vec![',', ';', '\t'],
        }
    }
}

/// Represents an error during CSV import
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct ImportError {
    pub row_number: usize,
    pub field: String,
    pub error_type: String,
    pub message: String,
    pub suggested_fix: Option<String>,
}

/// Validation result for a single question
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct QuestionValidation {
    pub row_number: usize,
    pub question: Option<CreateSoalRequest>,
    pub errors: Vec<ImportError>,
    pub warnings: Vec<ImportError>,
    pub is_valid: bool,
}

/// CSV import preview response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CsvImportPreview {
    pub total_rows: usize,
    pub valid_questions: Vec<QuestionValidation>,
    pub invalid_questions: Vec<QuestionValidation>,
    pub global_errors: Vec<String>,
    pub import_ready: bool,
    pub summary: ImportSummary,
}

/// Import summary statistics
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ImportSummary {
    pub total_rows: usize,
    pub valid_count: usize,
    pub invalid_count: usize,
    pub warning_count: usize,
    pub estimated_import_time_seconds: u32,
}

/// CSV import result
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CsvImportResult {
    pub success_count: i32,
    pub failed_count: i32,
    pub errors: Vec<ImportError>,
    pub processed_rows: usize,
}

/// Expected CSV header mapping
#[derive(Debug)]
pub struct CsvHeaderMapping {
    pub question_text: Option<usize>,
    pub option_1: Option<usize>,
    pub option_2: Option<usize>,
    pub option_3: Option<usize>,
    pub option_4: Option<usize>,
    pub option_5: Option<usize>,
    pub correct_answer: Option<usize>,
    pub solution: Option<usize>,
    pub module: Option<usize>,
    pub subject: Option<usize>,
    pub tag: Option<usize>,
    pub source_file: Option<usize>,
}

/// CSV import service
pub struct CsvImportService {
    config: CsvImportConfig,
}

impl CsvImportService {
    pub fn new(config: CsvImportConfig) -> Self {
        Self { config }
    }

    pub fn default() -> Self {
        Self {
            config: CsvImportConfig::default(),
        }
    }

    /// Parse CSV content and return preview
    pub async fn parse_csv_preview(&self, csv_content: &str) -> Result<CsvImportPreview, String> {
        // Detect delimiter
        let delimiter = self.detect_delimiter(csv_content)?;
        
        // Build CSV reader
        let mut reader = ReaderBuilder::new()
            .delimiter(delimiter as u8)
            .has_headers(true)
            .flexible(true)
            .from_reader(Cursor::new(csv_content));

        // Get headers and create mapping
        let headers = reader.headers()
            .map_err(|e| format!("Failed to read CSV headers: {}", e))?;
        
        let header_mapping = self.create_header_mapping(headers)?;
        
        // Validate header mapping
        if let Err(errors) = self.validate_headers(&header_mapping) {
            return Ok(CsvImportPreview {
                total_rows: 0,
                valid_questions: vec![],
                invalid_questions: vec![],
                global_errors: errors,
                import_ready: false,
                summary: ImportSummary {
                    total_rows: 0,
                    valid_count: 0,
                    invalid_count: 0,
                    warning_count: 0,
                    estimated_import_time_seconds: 0,
                },
            });
        }

        // Process rows
        let mut valid_questions = Vec::new();
        let mut invalid_questions = Vec::new();
        let mut row_number = 2; // Start from 2 (1 is header)

        for result in reader.records() {
            match result {
                Ok(record) => {
                    let validation = self.validate_csv_row(&record, &header_mapping, row_number);
                    
                    if validation.is_valid {
                        valid_questions.push(validation);
                    } else {
                        invalid_questions.push(validation);
                    }
                }
                Err(e) => {
                    invalid_questions.push(QuestionValidation {
                        row_number,
                        question: None,
                        errors: vec![ImportError {
                            row_number,
                            field: "csv_parsing".to_string(),
                            error_type: "parse_error".to_string(),
                            message: format!("Failed to parse CSV row: {}", e),
                            suggested_fix: Some("Check for malformed CSV data, quotes, or special characters".to_string()),
                        }],
                        warnings: vec![],
                        is_valid: false,
                    });
                }
            }
            row_number += 1;

            // Check limits
            if (valid_questions.len() + invalid_questions.len()) >= self.config.max_questions_per_import {
                break;
            }
        }

        let total_rows = valid_questions.len() + invalid_questions.len();
        let warning_count = valid_questions.iter().map(|q| q.warnings.len()).sum::<usize>() + 
                          invalid_questions.iter().map(|q| q.warnings.len()).sum::<usize>();

        Ok(CsvImportPreview {
            total_rows,
            valid_questions: valid_questions.clone(),
            invalid_questions: invalid_questions.clone(),
            global_errors: vec![],
            import_ready: !valid_questions.is_empty() && invalid_questions.is_empty(),
            summary: ImportSummary {
                total_rows,
                valid_count: valid_questions.len(),
                invalid_count: invalid_questions.len(),
                warning_count,
                estimated_import_time_seconds: std::cmp::max(1, (total_rows / 100) as u32), // Estimate 100 questions per second
            },
        })
    }

    /// Extract valid questions from preview for import
    pub fn extract_questions_from_preview(&self, preview: &CsvImportPreview) -> Vec<CreateSoalRequest> {
        preview.valid_questions
            .iter()
            .filter_map(|validation| validation.question.clone())
            .collect()
    }

    /// Detect CSV delimiter
    fn detect_delimiter(&self, content: &str) -> Result<char, String> {
        let first_line = content.lines().next()
            .ok_or_else(|| "Empty CSV file".to_string())?;

        let mut delimiter_counts = HashMap::new();
        
        for &delimiter in &self.config.allowed_delimiters {
            let count = first_line.chars().filter(|&c| c == delimiter).count();
            if count > 0 {
                delimiter_counts.insert(delimiter, count);
            }
        }

        delimiter_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(delimiter, _)| delimiter)
            .ok_or_else(|| "Could not detect CSV delimiter".to_string())
    }

    /// Create header mapping from CSV headers
    fn create_header_mapping(&self, headers: &csv::StringRecord) -> Result<CsvHeaderMapping, String> {
        let mut mapping = CsvHeaderMapping {
            question_text: None,
            option_1: None,
            option_2: None,
            option_3: None,
            option_4: None,
            option_5: None,
            correct_answer: None,
            solution: None,
            module: None,
            subject: None,
            tag: None,
            source_file: None,
        };

        for (index, header) in headers.iter().enumerate() {
            let header_lower = header.to_lowercase().trim().to_string();
            
            match header_lower.as_str() {
                "question_text" | "question" | "soal" => mapping.question_text = Some(index),
                "option_1" | "option1" | "opt1" | "a" => mapping.option_1 = Some(index),
                "option_2" | "option2" | "opt2" | "b" => mapping.option_2 = Some(index),
                "option_3" | "option3" | "opt3" | "c" => mapping.option_3 = Some(index),
                "option_4" | "option4" | "opt4" | "d" => mapping.option_4 = Some(index),
                "option_5" | "option5" | "opt5" | "e" => mapping.option_5 = Some(index),
                "correct_answer" | "answer" | "correct" | "jawaban" => mapping.correct_answer = Some(index),
                "solution" | "explanation" | "pembahasan" => mapping.solution = Some(index),
                "module" | "modul" => mapping.module = Some(index),
                "subject" | "pelajaran" | "mata_pelajaran" => mapping.subject = Some(index),
                "tag" | "tags" | "kategori" => mapping.tag = Some(index),
                "source_file" | "source" | "sumber" | "sumberfile" => mapping.source_file = Some(index),
                _ => {} // Unknown header, ignore
            }
        }

        Ok(mapping)
    }

    /// Validate header mapping has required fields
    fn validate_headers(&self, mapping: &CsvHeaderMapping) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if mapping.question_text.is_none() {
            errors.push("Required column 'question_text' not found. Expected headers: 'question_text', 'question', or 'soal'".to_string());
        }

        if mapping.option_1.is_none() {
            errors.push("Required column 'option_1' not found. Expected headers: 'option_1', 'option1', 'opt1', or 'a'".to_string());
        }

        if mapping.option_2.is_none() {
            errors.push("Required column 'option_2' not found. Expected headers: 'option_2', 'option2', 'opt2', or 'b'".to_string());
        }

        if mapping.correct_answer.is_none() {
            errors.push("Required column 'correct_answer' not found. Expected headers: 'correct_answer', 'answer', 'correct', or 'jawaban'".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate and convert a CSV row to CreateSoalRequest
    fn validate_csv_row(&self, record: &csv::StringRecord, mapping: &CsvHeaderMapping, row_number: usize) -> QuestionValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Helper function to get field value
        let get_field = |index: Option<usize>| -> String {
            index.and_then(|i| record.get(i)).unwrap_or("").trim().to_string()
        };

        // Extract fields
        let question_text = get_field(mapping.question_text);
        let opt1 = get_field(mapping.option_1);
        let opt2 = get_field(mapping.option_2);
        let opt3 = get_field(mapping.option_3);
        let opt4 = get_field(mapping.option_4);
        let opt5 = get_field(mapping.option_5);
        let correct_answer_raw = get_field(mapping.correct_answer);
        let solution = get_field(mapping.solution);
        let modul = get_field(mapping.module);
        let pelajaran = get_field(mapping.subject);
        let tag = get_field(mapping.tag);
        let sumberfile = get_field(mapping.source_file);

        // Validate question text
        if question_text.is_empty() {
            errors.push(ImportError {
                row_number,
                field: "question_text".to_string(),
                error_type: "required_field_empty".to_string(),
                message: "Question text is required and cannot be empty".to_string(),
                suggested_fix: Some("Add question text content".to_string()),
            });
        } else if question_text.len() < 10 {
            warnings.push(ImportError {
                row_number,
                field: "question_text".to_string(),
                error_type: "field_too_short".to_string(),
                message: "Question text is very short (< 10 characters)".to_string(),
                suggested_fix: Some("Consider adding more descriptive question text".to_string()),
            });
        } else if question_text.len() > 1000 {
            errors.push(ImportError {
                row_number,
                field: "question_text".to_string(),
                error_type: "field_too_long".to_string(),
                message: "Question text exceeds maximum length (1000 characters)".to_string(),
                suggested_fix: Some("Shorten the question text or split into multiple questions".to_string()),
            });
        }

        // Validate options
        if opt1.is_empty() {
            errors.push(ImportError {
                row_number,
                field: "option_1".to_string(),
                error_type: "required_field_empty".to_string(),
                message: "Option 1 is required and cannot be empty".to_string(),
                suggested_fix: Some("Add content for option 1".to_string()),
            });
        }

        if opt2.is_empty() {
            errors.push(ImportError {
                row_number,
                field: "option_2".to_string(),
                error_type: "required_field_empty".to_string(),
                message: "Option 2 is required and cannot be empty".to_string(),
                suggested_fix: Some("Add content for option 2".to_string()),
            });
        }

        // Validate correct answer
        let correct_answer = self.parse_correct_answer(&correct_answer_raw, row_number, &mut errors);

        // Count non-empty options
        let option_count = [&opt1, &opt2, &opt3, &opt4, &opt5]
            .iter()
            .filter(|opt| !opt.is_empty())
            .count();

        if option_count < 2 {
            errors.push(ImportError {
                row_number,
                field: "options".to_string(),
                error_type: "insufficient_options".to_string(),
                message: "At least 2 options are required".to_string(),
                suggested_fix: Some("Add at least 2 answer options".to_string()),
            });
        }

        // Validate correct answer against available options
        if let Some(ref answer) = correct_answer {
            let is_valid_answer = match answer.as_str() {
                "opt1" => !opt1.is_empty(),
                "opt2" => !opt2.is_empty(),
                "opt3" => !opt3.is_empty(),
                "opt4" => !opt4.is_empty(),
                "opt5" => !opt5.is_empty(),
                _ => false,
            };

            if !is_valid_answer {
                errors.push(ImportError {
                    row_number,
                    field: "correct_answer".to_string(),
                    error_type: "invalid_correct_answer".to_string(),
                    message: format!("Correct answer '{}' points to an empty or non-existent option", answer),
                    suggested_fix: Some("Ensure correct answer points to a non-empty option (1-5)".to_string()),
                });
            }
        }

        // Validate solution length
        if !solution.is_empty() && solution.len() < 10 {
            warnings.push(ImportError {
                row_number,
                field: "solution".to_string(),
                error_type: "field_too_short".to_string(),
                message: "Solution explanation is very short (< 10 characters)".to_string(),
                suggested_fix: Some("Consider adding more detailed explanation".to_string()),
            });
        }

        // Create question if valid
        let question = if errors.is_empty() {
            Some(CreateSoalRequest {
                soal: question_text,
                question_type: None, // defaults to "multiple_choice" in DAO
                opt1,
                opt2,
                opt3,
                opt4,
                opt5,
                correct_answer: correct_answer.unwrap_or_default(),
                solution,
                sumberfile: if sumberfile.is_empty() { None } else { Some(sumberfile) },
                modul: if modul.is_empty() { None } else { Some(modul) },
                pelajaran: if pelajaran.is_empty() { None } else { Some(pelajaran) },
                tag: if tag.is_empty() { None } else { Some(tag) },
            })
        } else {
            None
        };

        let is_valid = errors.is_empty();
        
        QuestionValidation {
            row_number,
            question,
            errors,
            warnings,
            is_valid,
        }
    }

    /// Parse correct answer from various formats
    fn parse_correct_answer(&self, raw_answer: &str, row_number: usize, errors: &mut Vec<ImportError>) -> Option<String> {
        if raw_answer.is_empty() {
            errors.push(ImportError {
                row_number,
                field: "correct_answer".to_string(),
                error_type: "required_field_empty".to_string(),
                message: "Correct answer is required and cannot be empty".to_string(),
                suggested_fix: Some("Specify correct answer as 1, 2, 3, 4, 5, A, B, C, D, E, or opt1, opt2, etc.".to_string()),
            });
            return None;
        }

        let answer_lower = raw_answer.to_lowercase().trim().to_string();
        
        match answer_lower.as_str() {
            "1" | "a" | "opt1" | "option1" | "option_1" => Some("opt1".to_string()),
            "2" | "b" | "opt2" | "option2" | "option_2" => Some("opt2".to_string()),
            "3" | "c" | "opt3" | "option3" | "option_3" => Some("opt3".to_string()),
            "4" | "d" | "opt4" | "option4" | "option_4" => Some("opt4".to_string()),
            "5" | "e" | "opt5" | "option5" | "option_5" => Some("opt5".to_string()),
            _ => {
                errors.push(ImportError {
                    row_number,
                    field: "correct_answer".to_string(),
                    error_type: "invalid_format".to_string(),
                    message: format!("Invalid correct answer format: '{}'", raw_answer),
                    suggested_fix: Some("Use 1-5, A-E, or opt1-opt5 format".to_string()),
                });
                None
            }
        }
    }

    /// Validate file size and content
    pub fn validate_file(&self, content: &[u8]) -> Result<(), String> {
        if content.len() > self.config.max_file_size {
            return Err(format!(
                "File size ({} bytes) exceeds maximum allowed size ({} bytes)",
                content.len(),
                self.config.max_file_size
            ));
        }

        // Check if content is valid UTF-8
        std::str::from_utf8(content)
            .map_err(|_| "File must be valid UTF-8 text".to_string())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delimiter_detection() {
        let service = CsvImportService::default();
        
        // Test comma delimiter
        let csv_comma = "question,option1,option2\n";
        assert_eq!(service.detect_delimiter(csv_comma).unwrap(), ',');
        
        // Test semicolon delimiter
        let csv_semicolon = "question;option1;option2\n";
        assert_eq!(service.detect_delimiter(csv_semicolon).unwrap(), ';');
        
        // Test tab delimiter  
        let csv_tab = "question\toption1\toption2\n";
        assert_eq!(service.detect_delimiter(csv_tab).unwrap(), '\t');
    }

    #[test]
    fn test_correct_answer_parsing() {
        let service = CsvImportService::default();
        let mut errors = Vec::new();
        
        // Test numeric format
        assert_eq!(service.parse_correct_answer("1", 1, &mut errors), Some("opt1".to_string()));
        assert_eq!(service.parse_correct_answer("3", 1, &mut errors), Some("opt3".to_string()));
        
        // Test letter format
        assert_eq!(service.parse_correct_answer("A", 1, &mut errors), Some("opt1".to_string()));
        assert_eq!(service.parse_correct_answer("C", 1, &mut errors), Some("opt3".to_string()));
        
        // Test option format
        assert_eq!(service.parse_correct_answer("opt2", 1, &mut errors), Some("opt2".to_string()));
        
        // Test invalid format
        assert_eq!(service.parse_correct_answer("invalid", 1, &mut errors), None);
        assert!(!errors.is_empty());
    }
}