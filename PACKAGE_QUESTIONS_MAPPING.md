# Package Questions Mapping Feature

## Overview

The Package Questions Mapping feature allows administrators to manage the relationship between quiz packages (`paket_soal`) and individual questions (`soal`) through a junction table `paket_soal_items`. This provides flexibility in organizing questions into different packages without duplicating question data.

## Database Schema

### Core Tables

#### `paket_soal_items` (Junction Table)
```sql
CREATE TABLE paket_soal_items (
    id INT PRIMARY KEY AUTO_INCREMENT,
    paket_soal_id INT NOT NULL,
    soal_id INT NOT NULL,
    FOREIGN KEY (paket_soal_id) REFERENCES paket_soal(id) ON DELETE CASCADE,
    FOREIGN KEY (soal_id) REFERENCES soal(id) ON DELETE CASCADE,
    UNIQUE KEY unique_mapping (paket_soal_id, soal_id)
);
```

#### Related Tables
- `paket_soal`: Contains quiz packages/sets
- `soal`: Contains individual questions
- `kategori_soal`: Contains question categories

## Backend API Endpoints

### Base URL: `/api/admin/paket-soal`

#### 1. Get Package Questions
**GET** `/{paket_soal_id}/questions`

Returns all questions currently mapped to a specific package with details.

**Response:**
```json
[
  {
    "id": 1,
    "paket_soal_id": 5,
    "soal_id": 123,
    "soal_pertanyaan": "What is the capital of Indonesia?",
    "soal_kategori": "Geography",
    "soal_tingkat_kesulitan": "Easy"
  }
]
```

#### 2. Get Available Questions
**GET** `/{paket_soal_id}/available-questions`

Returns all questions with mapping status for the specified package.

**Response:**
```json
[
  {
    "id": 123,
    "pertanyaan": "What is the capital of Indonesia?",
    "kategori": "Geography",
    "tingkat_kesulitan": "Easy",
    "is_mapped": true
  },
  {
    "id": 124,
    "pertanyaan": "What is 2 + 2?",
    "kategori": "Mathematics",
    "tingkat_kesulitan": "Easy",
    "is_mapped": false
  }
]
```

#### 3. Map Questions to Package
**POST** `/{paket_soal_id}/map-questions`

Maps multiple questions to a package.

**Request Body:**
```json
{
  "paket_soal_id": 5,
  "soal_ids": [124, 125, 126]
}
```

**Response:**
```json
{
  "success": true,
  "message": "3 questions mapped successfully",
  "mapped_count": 3
}
```

#### 4. Remove Questions from Package
**DELETE** `/{paket_soal_id}/unmap-questions`

Removes multiple questions from a package mapping.

**Request Body:**
```json
{
  "paket_soal_id": 5,
  "soal_ids": [124, 125]
}
```

**Response:**
```json
{
  "success": true,
  "message": "2 questions unmapped successfully",
  "mapped_count": 2
}
```

### Base URL: `/api/admin/paket-soal-items`

#### 5. Delete Specific Mapping
**DELETE** `/{id}`

Deletes a specific mapping by its ID.

**Response:**
```json
"Mapping removed successfully"
```

## Frontend Implementation

### Components

#### `PackageQuestionMapping`
Main component for managing package-question mappings with:
- Tabbed interface (Mapped Questions / All Questions)
- Bulk selection capabilities
- Real-time mapping/unmapping
- Search and filtering

#### Key Features
- **Dual View Mode**: Switch between mapped questions and all available questions
- **Bulk Operations**: Select multiple questions for batch mapping/unmapping
- **Real-time Updates**: Immediate UI updates after operations using React Query
- **Error Handling**: Comprehensive error handling with user-friendly messages
- **Loading States**: Visual feedback during async operations

### Navigation Integration

The mapping feature is accessible from the packages table through a "Map" button:
- Package List → Click "Map" → Opens mapping interface
- URL pattern: `/admin/packages/{id}/mapping?name={package_name}`

## Usage Guide

### For Administrators

1. **Accessing the Feature**
   - Navigate to Admin → Packages
   - Find the package you want to manage
   - Click the "Map" button in the Actions column

2. **Mapping Questions**
   - Switch to "All Questions" tab
   - Select unmapped questions using checkboxes
   - Click "Map Selected" to add them to the package

3. **Removing Questions**
   - Stay on "Mapped Questions" tab
   - Select questions you want to remove
   - Click "Remove Selected" to unmap them

4. **Individual Operations**
   - Use the trash icon next to each mapped question for individual removal
   - Questions are marked with a green "Mapped" badge in the All Questions view

### Best Practices

1. **Question Organization**: Group related questions into logical packages
2. **Avoid Duplication**: Use the mapping system instead of copying questions
3. **Regular Maintenance**: Periodically review and update package contents
4. **Testing**: Always verify package contents before publishing

## Technical Implementation Details

### Backend Architecture

#### Models (`src/model/paket_soal_items.rs`)
- `PaketSoalItem`: Basic mapping structure
- `PaketSoalItemWithDetails`: Extended mapping with question details
- `AvailableSoal`: Question with mapping status
- `MappingRequest/Response`: API request/response structures

#### Controller (`src/controller/admin_paket_soal_items_controller.rs`)
- Transaction-based operations for data consistency
- Comprehensive error handling
- SQL joins for efficient data retrieval
- Bulk operations support

#### Key SQL Patterns
```sql
-- Get mapped questions with details
SELECT 
    psi.id,
    psi.paket_soal_id,
    psi.soal_id,
    s.pertanyaan as soal_pertanyaan,
    ks.name as soal_kategori,
    s.tingkat_kesulitan as soal_tingkat_kesulitan
FROM paket_soal_items psi
JOIN soal s ON psi.soal_id = s.id
LEFT JOIN kategori_soal ks ON s.kategori_id = ks.id
WHERE psi.paket_soal_id = ?

-- Get all questions with mapping status
SELECT 
    s.id,
    s.pertanyaan,
    ks.name as kategori,
    s.tingkat_kesulitan,
    CASE WHEN psi.soal_id IS NOT NULL THEN 1 ELSE 0 END as is_mapped
FROM soal s
LEFT JOIN kategori_soal ks ON s.kategori_id = ks.id
LEFT JOIN paket_soal_items psi ON s.id = psi.soal_id AND psi.paket_soal_id = ?
WHERE s.deleted_at IS NULL
```

### Frontend Architecture

#### Services (`src/services/packageMapping.ts`)
- `PackageMappingService`: Centralized API communication
- TypeScript interfaces for type safety
- Error handling and logging

#### Components
- **PackageQuestionMapping**: Main container component
- **MappedQuestionRow**: Individual mapped question display
- **AvailableQuestionRow**: Individual available question display

#### State Management
- React Query for server state
- Local state for UI interactions (selections, view mode)
- Optimistic updates with rollback on failure

## Error Handling

### Backend Errors
- **400 Bad Request**: Invalid input data
- **404 Not Found**: Package or mapping not found
- **500 Internal Server Error**: Database or server issues

### Frontend Error Handling
- Toast notifications for user feedback
- Graceful degradation on API failures
- Retry mechanisms for transient failures

## Performance Considerations

### Backend Optimizations
- Database indexes on foreign keys
- Efficient SQL joins
- Transaction batching for bulk operations
- Connection pooling

### Frontend Optimizations
- React Query caching
- Virtualized lists for large datasets
- Debounced search inputs
- Lazy loading of component data

## Security

### Authentication & Authorization
- JWT-based admin authentication
- Role-based access control
- Request validation and sanitization

### Data Validation
- Input sanitization on all endpoints
- Foreign key constraint validation
- Duplicate mapping prevention

## Testing

### Backend Testing
```bash
# Run backend tests
cargo test

# Specific endpoint testing
curl -X GET "http://localhost:8787/api/admin/paket-soal/1/questions" \
     -H "Authorization: Bearer <admin_token>"
```

### Frontend Testing
- Component unit tests with React Testing Library
- Integration tests for API interactions
- E2E tests for complete workflows

## Troubleshooting

### Common Issues

1. **"No questions available"**
   - Check if questions exist in the database
   - Verify questions are not soft-deleted (`deleted_at` is NULL)

2. **"Mapping failed"**
   - Ensure both package and questions exist
   - Check for duplicate mappings
   - Verify database constraints

3. **"500 Internal Server Error"**
   - Check backend logs for detailed error messages
   - Verify database connection and permissions
   - Confirm table structure matches expected schema

### Debug Commands
```sql
-- Check package exists
SELECT * FROM paket_soal WHERE id = ?;

-- Check question exists
SELECT * FROM soal WHERE id = ? AND deleted_at IS NULL;

-- Check existing mappings
SELECT * FROM paket_soal_items WHERE paket_soal_id = ?;

-- Check for constraint violations
SHOW CREATE TABLE paket_soal_items;
```

## Future Enhancements

### Planned Features
1. **Question Templates**: Reusable question sets
2. **Advanced Filtering**: Filter by difficulty, category, tags
3. **Bulk Import**: CSV/Excel import for mass mapping
4. **Analytics**: Package performance metrics
5. **Version Control**: Track mapping changes over time

### API Extensions
1. **Batch Operations**: Multiple packages at once
2. **Search Integration**: Full-text search across questions
3. **Export Features**: Generate package reports
4. **Scheduling**: Automated package updates

## Conclusion

The Package Questions Mapping feature provides a robust foundation for managing quiz content with flexibility and scalability. The implementation follows best practices for both backend API design and frontend user experience, ensuring maintainability and extensibility for future enhancements.