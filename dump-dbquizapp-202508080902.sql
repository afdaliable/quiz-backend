-- MySQL dump 10.13  Distrib 9.4.0, for macos15.4 (arm64)
--
-- Host: 100.124.237.85    Database: dbquizapp
-- ------------------------------------------------------
-- Server version	8.0.41-0ubuntu0.24.04.1

/*!40101 SET @OLD_CHARACTER_SET_CLIENT=@@CHARACTER_SET_CLIENT */;
/*!40101 SET @OLD_CHARACTER_SET_RESULTS=@@CHARACTER_SET_RESULTS */;
/*!40101 SET @OLD_COLLATION_CONNECTION=@@COLLATION_CONNECTION */;
/*!50503 SET NAMES utf8mb4 */;
/*!40103 SET @OLD_TIME_ZONE=@@TIME_ZONE */;
/*!40103 SET TIME_ZONE='+00:00' */;
/*!40014 SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0 */;
/*!40014 SET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0 */;
/*!40101 SET @OLD_SQL_MODE=@@SQL_MODE, SQL_MODE='NO_AUTO_VALUE_ON_ZERO' */;
/*!40111 SET @OLD_SQL_NOTES=@@SQL_NOTES, SQL_NOTES=0 */;

--
-- Table structure for table `harga_paket`
--

DROP TABLE IF EXISTS `harga_paket`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `harga_paket` (
  `id` int NOT NULL AUTO_INCREMENT,
  `id_paket_soal` int NOT NULL,
  `koin` int NOT NULL DEFAULT '0',
  `harga` int NOT NULL DEFAULT '0',
  `is_free` tinyint(1) NOT NULL DEFAULT '0',
  `created_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `dbquizapp_harga_paket_id_uindex` (`id`),
  KEY `dbquizapp_harga_paket_id_paket_soal_idx` (`id_paket_soal`),
  CONSTRAINT `harga_paket_ibfk_1` FOREIGN KEY (`id_paket_soal`) REFERENCES `paket_soal` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB AUTO_INCREMENT=2 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `harga_paket`
--

LOCK TABLES `harga_paket` WRITE;
/*!40000 ALTER TABLE `harga_paket` DISABLE KEYS */;
INSERT INTO `harga_paket` VALUES (1,1,10,12000,0,'2025-03-08 03:58:47','2025-03-08 04:03:16');
/*!40000 ALTER TABLE `harga_paket` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `jawaban_user`
--

DROP TABLE IF EXISTS `jawaban_user`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `jawaban_user` (
  `id` int NOT NULL AUTO_INCREMENT,
  `soal_id` int DEFAULT NULL,
  `user_id` int DEFAULT NULL,
  `jawaban` varchar(500) DEFAULT NULL,
  `is_correct` tinyint(1) DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `dbquizapp_jawaban_user_id_uindex` (`id`),
  KEY `dbquizapp_jawaban_user_soal_id_idx` (`soal_id`),
  KEY `dbquizapp_jawaban_user_user_id_idx` (`user_id`),
  CONSTRAINT `jawaban_user_ibfk_1` FOREIGN KEY (`soal_id`) REFERENCES `soal` (`id`),
  CONSTRAINT `jawaban_user_ibfk_2` FOREIGN KEY (`user_id`) REFERENCES `users_history` (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `jawaban_user`
--

LOCK TABLES `jawaban_user` WRITE;
/*!40000 ALTER TABLE `jawaban_user` DISABLE KEYS */;
/*!40000 ALTER TABLE `jawaban_user` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `kategori_soal`
--

DROP TABLE IF EXISTS `kategori_soal`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `kategori_soal` (
  `id` int NOT NULL AUTO_INCREMENT,
  `nama_kategori` varchar(255) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `nama_kategori` (`nama_kategori`),
  UNIQUE KEY `dbquizapp_kategori_soal_id_uindex` (`id`),
  UNIQUE KEY `dbquizapp_kategori_soal_nama_kategori_uindex` (`nama_kategori`)
) ENGINE=InnoDB AUTO_INCREMENT=7 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `kategori_soal`
--

LOCK TABLES `kategori_soal` WRITE;
/*!40000 ALTER TABLE `kategori_soal` DISABLE KEYS */;
INSERT INTO `kategori_soal` VALUES (6,'BEASISWA'),(4,'CPNS'),(2,'KEDINASAN'),(3,'MANDIRI'),(5,'PPPK'),(1,'UTBK');
/*!40000 ALTER TABLE `kategori_soal` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `license_codes`
--

DROP TABLE IF EXISTS `license_codes`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `license_codes` (
  `id` int NOT NULL AUTO_INCREMENT,
  `license_code` varchar(255) NOT NULL COMMENT 'License code from Mayar',
  `user_id` char(36) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT 'User ID who activated this license',
  `plan_id` int NOT NULL COMMENT 'Premium plan ID',
  `status` enum('active','expired','cancelled') DEFAULT 'active',
  `transaction_id` varchar(255) DEFAULT NULL COMMENT 'Transaction ID from Mayar',
  `product_id` varchar(255) NOT NULL COMMENT 'Product ID from Mayar',
  `customer_id` varchar(255) DEFAULT NULL COMMENT 'Customer ID from Mayar',
  `customer_name` varchar(255) DEFAULT NULL COMMENT 'Customer name from Mayar',
  `customer_email` varchar(255) DEFAULT NULL COMMENT 'Customer email from Mayar',
  `expired_at` timestamp NULL DEFAULT NULL COMMENT 'Expiration date from Mayar',
  `activation_limit` varchar(255) DEFAULT NULL COMMENT 'Activation limit from Mayar',
  `use_count` int DEFAULT NULL COMMENT 'Use count from Mayar',
  `created_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `license_data` text COMMENT 'Full JSON response from Mayar license verification',
  PRIMARY KEY (`id`),
  UNIQUE KEY `license_code` (`license_code`),
  UNIQUE KEY `dbquizapp_license_codes_id_uindex` (`id`),
  UNIQUE KEY `dbquizapp_license_codes_license_code_uindex` (`license_code`),
  KEY `dbquizapp_license_codes_user_id_idx` (`user_id`),
  KEY `dbquizapp_license_codes_plan_id_idx` (`plan_id`),
  KEY `dbquizapp_license_codes_status_idx` (`status`),
  CONSTRAINT `license_codes_ibfk_1` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`),
  CONSTRAINT `license_codes_ibfk_2` FOREIGN KEY (`plan_id`) REFERENCES `premium_plans` (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=5 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `license_codes`
--

LOCK TABLES `license_codes` WRITE;
/*!40000 ALTER TABLE `license_codes` DISABLE KEYS */;
INSERT INTO `license_codes` VALUES (1,'2136D65C95366AB5','6a8c7e6e-ffba-4e67-a644-0657d2a1bd92',5,'active','b7a0b303-bba7-44f7-9f72-7fbc0f87487e','aedda011-50a6-4839-9109-3e413ad5b887','3680f8bc-96fd-4518-acce-083a0c98217e','Afdhal Kurniawan','arazakibsr@gmail.com','2025-04-16 07:26:48',NULL,NULL,'2025-03-16 07:54:05','2025-03-16 07:54:05','{\"statusCode\":200,\"isLicenseActive\":true,\"licenseCode\":{\"licenseCode\":\"2136D65C95366AB5\",\"status\":\"ISSUED\",\"expiredAt\":\"2025-04-16T07:26:47.999Z\",\"transactionId\":\"b7a0b303-bba7-44f7-9f72-7fbc0f87487e\",\"productId\":\"aedda011-50a6-4839-9109-3e413ad5b887\",\"customerId\":\"3680f8bc-96fd-4518-acce-083a0c98217e\",\"customerName\":\"Afdhal Kurniawan\",\"customerEmail\":\"arazakibsr@gmail.com\",\"activationLimit\":null,\"useCount\":null,\"createdAt\":\"2025-03-16T07:26:50.577Z\",\"updatedAt\":\"2025-03-16T07:26:50.801Z\",\"membershipTierId\":\"727c23b5-6990-40da-8d0e-d9dcc84ca779\",\"membershipTierName\":\"Special Plan\"}}'),(2,'0C6A09CFA61BBB98','ba46297e-764d-4949-82d6-45623c60c325',5,'active','daf3ec80-d0e4-491a-8dd6-ae0d537d0317','aedda011-50a6-4839-9109-3e413ad5b887','4f363acc-ed12-4c77-be7a-d554c279cb6b','afdal sitp','afdalsitp@gmail.com','2025-04-16 05:53:47',NULL,NULL,'2025-03-16 08:29:26','2025-03-16 08:29:26','{\"statusCode\":200,\"isLicenseActive\":true,\"licenseCode\":{\"licenseCode\":\"0C6A09CFA61BBB98\",\"status\":\"ISSUED\",\"expiredAt\":\"2025-04-16T05:53:47.090Z\",\"transactionId\":\"daf3ec80-d0e4-491a-8dd6-ae0d537d0317\",\"productId\":\"aedda011-50a6-4839-9109-3e413ad5b887\",\"customerId\":\"4f363acc-ed12-4c77-be7a-d554c279cb6b\",\"customerName\":\"Afdhal Kurniawan\",\"customerEmail\":\"afdalsitp@gmail.com\",\"activationLimit\":null,\"useCount\":null,\"createdAt\":\"2025-03-16T05:53:49.621Z\",\"updatedAt\":\"2025-03-16T05:53:49.826Z\",\"membershipTierId\":\"727c23b5-6990-40da-8d0e-d9dcc84ca779\",\"membershipTierName\":\"Special Plan\"}}'),(3,'F40972BF84AF9732','ba46297e-764d-4949-82d6-45623c60c325',1,'active','3e1a267c-2fdd-455b-b280-80eb2786c221','8a0fbcb4-412f-41f1-b46b-7cb3e21404ac','4f363acc-ed12-4c77-be7a-d554c279cb6b','afdal sitp','afdalsitp@gmail.com','2025-04-16 13:51:51',NULL,NULL,'2025-03-16 13:59:04','2025-03-16 13:59:04','{\"statusCode\":200,\"isLicenseActive\":true,\"licenseCode\":{\"licenseCode\":\"F40972BF84AF9732\",\"status\":\"ISSUED\",\"expiredAt\":\"2025-04-16T13:51:51.136Z\",\"transactionId\":\"3e1a267c-2fdd-455b-b280-80eb2786c221\",\"productId\":\"8a0fbcb4-412f-41f1-b46b-7cb3e21404ac\",\"customerId\":\"4f363acc-ed12-4c77-be7a-d554c279cb6b\",\"customerName\":\"Afdhal Kurniawan\",\"customerEmail\":\"afdalsitp@gmail.com\",\"activationLimit\":null,\"useCount\":null,\"createdAt\":\"2025-03-16T13:51:53.430Z\",\"updatedAt\":\"2025-03-16T13:51:53.635Z\",\"membershipTierId\":\"d76a7867-7c95-486d-bb93-fc6d13785d0b\",\"membershipTierName\":\"Silver Plan\"}}'),(4,'288129EF19294697','5f63d46f-122f-4b6d-a350-528414b3703c',1,'active','72d73023-5abf-4055-8bfc-10b0e95584a8','8a0fbcb4-412f-41f1-b46b-7cb3e21404ac','21cf0739-1821-4f09-92f2-3bdf45d8c7dd','afdhal kurniawan','afdhal.kurniawan28@gmail.com','2025-04-17 04:42:23',NULL,NULL,'2025-03-17 04:43:15','2025-03-17 04:43:15','{\"statusCode\":200,\"isLicenseActive\":true,\"licenseCode\":{\"licenseCode\":\"288129EF19294697\",\"status\":\"ISSUED\",\"expiredAt\":\"2025-04-17T04:42:22.968Z\",\"transactionId\":\"72d73023-5abf-4055-8bfc-10b0e95584a8\",\"productId\":\"8a0fbcb4-412f-41f1-b46b-7cb3e21404ac\",\"customerId\":\"21cf0739-1821-4f09-92f2-3bdf45d8c7dd\",\"customerName\":\"Afdhal Kurniawan\",\"customerEmail\":\"afdhal.kurniawan28@gmail.com\",\"activationLimit\":null,\"useCount\":null,\"createdAt\":\"2025-03-17T04:42:25.302Z\",\"updatedAt\":\"2025-03-17T04:42:25.510Z\",\"membershipTierId\":\"d76a7867-7c95-486d-bb93-fc6d13785d0b\",\"membershipTierName\":\"Silver Plan\"}}');
/*!40000 ALTER TABLE `license_codes` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `paket_soal`
--

DROP TABLE IF EXISTS `paket_soal`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `paket_soal` (
  `id` int NOT NULL AUTO_INCREMENT,
  `nama_paket_soal` varchar(255) NOT NULL,
  `kategori_id` int DEFAULT NULL,
  `is_premium` tinyint(1) NOT NULL DEFAULT '0',
  PRIMARY KEY (`id`),
  UNIQUE KEY `dbquizapp_paket_soal_id_uindex` (`id`),
  KEY `dbquizapp_paket_soal_kategori_id_idx` (`kategori_id`),
  CONSTRAINT `paket_soal_ibfk_1` FOREIGN KEY (`kategori_id`) REFERENCES `kategori_soal` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB AUTO_INCREMENT=8 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `paket_soal`
--

LOCK TABLES `paket_soal` WRITE;
/*!40000 ALTER TABLE `paket_soal` DISABLE KEYS */;
INSERT INTO `paket_soal` VALUES (1,'paket utbk 2024',1,0),(2,'paket_kedinasan_1',2,0),(3,'paket_utbk_2',1,1),(4,'paket_mandiri_1',3,0),(5,'paket_cpns_1',4,1),(6,'paket_pppk_1',5,0),(7,'paket_beasiswa_1',6,1);
/*!40000 ALTER TABLE `paket_soal` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `paket_soal_items`
--

DROP TABLE IF EXISTS `paket_soal_items`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `paket_soal_items` (
  `id` int NOT NULL AUTO_INCREMENT,
  `paket_soal_id` int DEFAULT NULL,
  `soal_id` int DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `dbquizapp_paket_soal_items_id_uindex` (`id`),
  KEY `dbquizapp_paket_soal_items_paket_soal_id_idx` (`paket_soal_id`),
  KEY `dbquizapp_paket_soal_items_soal_id_idx` (`soal_id`),
  CONSTRAINT `paket_soal_items_ibfk_1` FOREIGN KEY (`paket_soal_id`) REFERENCES `paket_soal` (`id`) ON DELETE CASCADE,
  CONSTRAINT `paket_soal_items_ibfk_2` FOREIGN KEY (`soal_id`) REFERENCES `soal` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB AUTO_INCREMENT=24 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `paket_soal_items`
--

LOCK TABLES `paket_soal_items` WRITE;
/*!40000 ALTER TABLE `paket_soal_items` DISABLE KEYS */;
INSERT INTO `paket_soal_items` VALUES (1,1,1),(2,1,3),(3,2,2),(4,2,1),(5,3,2),(6,2,4),(7,1,2),(8,1,4),(9,1,5),(10,1,6),(11,1,7),(12,1,8),(13,1,607),(14,4,5),(15,5,6),(16,6,7),(17,7,8),(18,7,6),(19,7,1),(20,7,3),(21,5,1),(22,5,3),(23,6,4);
/*!40000 ALTER TABLE `paket_soal_items` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `payment_transactions`
--

DROP TABLE IF EXISTS `payment_transactions`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `payment_transactions` (
  `id` int NOT NULL AUTO_INCREMENT,
  `user_id` char(36) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL,
  `plan_id` int NOT NULL,
  `amount` decimal(10,2) NOT NULL,
  `transaction_id` varchar(255) NOT NULL COMMENT 'Transaction ID from payment gateway',
  `payment_link` varchar(255) NOT NULL,
  `status` enum('pending','completed','failed','expired') DEFAULT 'pending',
  `payment_method` varchar(100) DEFAULT NULL,
  `payment_details` text COMMENT 'JSON with payment details',
  `webhook_data` text COMMENT 'JSON with webhook data',
  `created_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `dbquizapp_payment_transactions_id_uindex` (`id`),
  UNIQUE KEY `dbquizapp_payment_transactions_transaction_id_uindex` (`transaction_id`),
  KEY `dbquizapp_payment_transactions_user_id_idx` (`user_id`),
  KEY `dbquizapp_payment_transactions_plan_id_idx` (`plan_id`),
  KEY `dbquizapp_payment_transactions_status_idx` (`status`),
  CONSTRAINT `payment_transactions_ibfk_1` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`),
  CONSTRAINT `payment_transactions_ibfk_2` FOREIGN KEY (`plan_id`) REFERENCES `premium_plans` (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=4 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `payment_transactions`
--

LOCK TABLES `payment_transactions` WRITE;
/*!40000 ALTER TABLE `payment_transactions` DISABLE KEYS */;
INSERT INTO `payment_transactions` VALUES (1,'6a8c7e6e-ffba-4e67-a644-0657d2a1bd92',5,75000.00,'abd221c3-9ba7-4550-8874-12882a58cf9a','https://canducation.myr.id/pl/jwh06lnoxx','pending',NULL,NULL,NULL,'2025-03-15 13:51:52','2025-03-15 13:51:52'),(2,'6a8c7e6e-ffba-4e67-a644-0657d2a1bd92',5,1000.00,'72f7efba-d605-4b64-a873-38d8d0ac89fc','https://canducation.myr.id/pl/j9wxguhx7i','pending',NULL,NULL,NULL,'2025-03-15 13:53:45','2025-03-15 13:53:45'),(3,'6a8c7e6e-ffba-4e67-a644-0657d2a1bd92',5,1000.00,'687dff05-8986-4d64-ad7e-c01eb0f28483','https://canducation.myr.id/pl/qzpbzsjnny','pending',NULL,NULL,NULL,'2025-03-15 14:51:21','2025-03-15 14:51:21');
/*!40000 ALTER TABLE `payment_transactions` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `premium_plans`
--

DROP TABLE IF EXISTS `premium_plans`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `premium_plans` (
  `id` int NOT NULL AUTO_INCREMENT,
  `name` varchar(100) NOT NULL,
  `description` text NOT NULL,
  `price` double NOT NULL,
  `duration_days` int NOT NULL COMMENT 'Duration in days, 0 for lifetime',
  `is_lifetime` tinyint(1) NOT NULL DEFAULT '0',
  `features` text NOT NULL COMMENT 'JSON array of features',
  `created_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `mayar_product_id` varchar(255) DEFAULT NULL COMMENT 'Product ID from Mayar',
  `mayar_link_payment` varchar(255) DEFAULT NULL COMMENT 'Link to Mayar payment page',
  PRIMARY KEY (`id`),
  UNIQUE KEY `dbquizapp_premium_plans_id_uindex` (`id`),
  UNIQUE KEY `dbquizapp_premium_plans_name_uindex` (`name`)
) ENGINE=InnoDB AUTO_INCREMENT=6 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `premium_plans`
--

LOCK TABLES `premium_plans` WRITE;
/*!40000 ALTER TABLE `premium_plans` DISABLE KEYS */;
INSERT INTO `premium_plans` VALUES (1,'Silver Plan','Basic premium features with limited access',99000,30,0,'[\"Access to basic premium quizzes\", \"Priority support\", \"Ad-free experience\"]','2025-03-15 04:11:13','2025-03-16 12:02:54','8a0fbcb4-412f-41f1-b46b-7cb3e21404ac','https://canducation.myr.id/m/silver-plan'),(2,'Gold Plan','Advanced premium features with extended access',199000,90,0,'[\"Access to all premium quizzes\", \"Priority support\", \"Ad-free experience\", \"Detailed performance analytics\", \"Downloadable quiz reports\"]','2025-03-15 04:11:13','2025-03-15 04:11:13',NULL,NULL),(3,'Platinum Plan','Complete premium package with all features',299000,180,0,'[\"Access to all premium quizzes\", \"Priority support\", \"Ad-free experience\", \"Detailed performance analytics\", \"Downloadable quiz reports\", \"Personalized learning path\", \"Expert consultation\"]','2025-03-15 04:11:13','2025-03-15 04:11:13',NULL,NULL),(4,'Ultimate Plan','Lifetime access to all premium features',999000,0,1,'[\"Lifetime access to all premium quizzes\", \"Priority support\", \"Ad-free experience\", \"Detailed performance analytics\", \"Downloadable quiz reports\", \"Personalized learning path\", \"Expert consultation\", \"Early access to new features\"]','2025-03-15 04:11:13','2025-03-15 04:11:13',NULL,NULL),(5,'Special Plan','Limited time offer',1000,60,0,'[\"Access to 20 premium quizzes\",\"Basic analytics\",\"Special badge\"]','2025-03-15 07:48:50','2025-03-16 07:14:18','aedda011-50a6-4839-9109-3e413ad5b887','https://canducation.myr.id/m/special-plan');
/*!40000 ALTER TABLE `premium_plans` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `premium_quiz_access`
--

DROP TABLE IF EXISTS `premium_quiz_access`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `premium_quiz_access` (
  `id` int NOT NULL AUTO_INCREMENT,
  `paket_soal_id` int NOT NULL,
  `min_plan_id` int NOT NULL COMMENT 'Minimum plan ID required to access this quiz',
  `created_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `dbquizapp_premium_quiz_access_id_uindex` (`id`),
  UNIQUE KEY `dbquizapp_premium_quiz_access_paket_soal_id_uindex` (`paket_soal_id`),
  KEY `dbquizapp_premium_quiz_access_min_plan_id_idx` (`min_plan_id`),
  CONSTRAINT `premium_quiz_access_ibfk_1` FOREIGN KEY (`paket_soal_id`) REFERENCES `paket_soal` (`id`),
  CONSTRAINT `premium_quiz_access_ibfk_2` FOREIGN KEY (`min_plan_id`) REFERENCES `premium_plans` (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=4 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `premium_quiz_access`
--

LOCK TABLES `premium_quiz_access` WRITE;
/*!40000 ALTER TABLE `premium_quiz_access` DISABLE KEYS */;
INSERT INTO `premium_quiz_access` VALUES (1,1,1,'2025-03-15 04:13:41','2025-03-15 04:13:41'),(2,2,2,'2025-03-15 04:14:15','2025-03-15 04:14:15'),(3,3,1,'2025-03-15 04:15:01','2025-03-15 04:15:01');
/*!40000 ALTER TABLE `premium_quiz_access` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `quiz_answers`
--

DROP TABLE IF EXISTS `quiz_answers`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `quiz_answers` (
  `id` int NOT NULL AUTO_INCREMENT,
  `session_id` varchar(36) NOT NULL,
  `soal_id` int NOT NULL,
  `selected_option` int NOT NULL,
  `is_correct` tinyint(1) DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `dbquizapp_quiz_answers_id_uindex` (`id`),
  KEY `idx_quiz_answers_session` (`session_id`,`soal_id`),
  KEY `dbquizapp_quiz_answers_soal_id_idx` (`soal_id`),
  KEY `dbquizapp_quiz_answers_session_id_soal_id_idx` (`session_id`,`soal_id`),
  CONSTRAINT `quiz_answers_ibfk_1` FOREIGN KEY (`session_id`) REFERENCES `quiz_sessions` (`id`),
  CONSTRAINT `quiz_answers_ibfk_2` FOREIGN KEY (`soal_id`) REFERENCES `soal` (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `quiz_answers`
--

LOCK TABLES `quiz_answers` WRITE;
/*!40000 ALTER TABLE `quiz_answers` DISABLE KEYS */;
/*!40000 ALTER TABLE `quiz_answers` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `quiz_sessions`
--

DROP TABLE IF EXISTS `quiz_sessions`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `quiz_sessions` (
  `id` varchar(500) NOT NULL,
  `user_id` varchar(500) NOT NULL,
  `paket_soal_id` int NOT NULL,
  `start_time` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `duration` int NOT NULL,
  `status` enum('ongoing','completed') DEFAULT 'ongoing',
  PRIMARY KEY (`id`),
  UNIQUE KEY `dbquizapp_quiz_sessions_id_uindex` (`id`),
  KEY `dbquizapp_quiz_sessions_paket_soal_id_idx` (`paket_soal_id`),
  KEY `dbquizapp_quiz_sessions_user_id_status_idx` (`user_id`,`status`),
  CONSTRAINT `quiz_sessions_ibfk_1` FOREIGN KEY (`paket_soal_id`) REFERENCES `paket_soal` (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `quiz_sessions`
--

LOCK TABLES `quiz_sessions` WRITE;
/*!40000 ALTER TABLE `quiz_sessions` DISABLE KEYS */;
/*!40000 ALTER TABLE `quiz_sessions` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `roles`
--

DROP TABLE IF EXISTS `roles`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `roles` (
  `id` int NOT NULL AUTO_INCREMENT,
  `name` varchar(50) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `name` (`name`),
  UNIQUE KEY `dbquizapp_roles_id_uindex` (`id`),
  UNIQUE KEY `dbquizapp_roles_name_uindex` (`name`)
) ENGINE=InnoDB AUTO_INCREMENT=4 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `roles`
--

LOCK TABLES `roles` WRITE;
/*!40000 ALTER TABLE `roles` DISABLE KEYS */;
INSERT INTO `roles` VALUES (1,'admin'),(2,'member'),(3,'pembuat_soal');
/*!40000 ALTER TABLE `roles` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `sessions`
--

DROP TABLE IF EXISTS `sessions`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `sessions` (
  `id` int NOT NULL AUTO_INCREMENT,
  `user_id` char(36) NOT NULL,
  `token` varchar(255) NOT NULL,
  `expires_at` timestamp NOT NULL,
  `ip_address` varchar(45) DEFAULT NULL,
  `user_agent` text,
  `created_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `token` (`token`),
  UNIQUE KEY `dbquizapp_sessions_id_uindex` (`id`),
  UNIQUE KEY `dbquizapp_sessions_token_uindex` (`token`),
  KEY `idx_sessions_user_id` (`user_id`),
  KEY `idx_sessions_token` (`token`),
  KEY `dbquizapp_sessions_user_id_idx` (`user_id`),
  CONSTRAINT `sessions_ibfk_1` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB AUTO_INCREMENT=83 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `sessions`
--

LOCK TABLES `sessions` WRITE;
/*!40000 ALTER TABLE `sessions` DISABLE KEYS */;
INSERT INTO `sessions` VALUES (82,'6a8c7e6e-ffba-4e67-a644-0657d2a1bd92','766b499c-d387-4a6f-9172-213275b5937b','2025-07-24 12:29:28','127.0.0.1','Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36','2025-07-23 12:29:27','2025-07-23 12:29:27');
/*!40000 ALTER TABLE `sessions` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `soal`
--

DROP TABLE IF EXISTS `soal`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `soal` (
  `id` int NOT NULL AUTO_INCREMENT,
  `soal` text NOT NULL,
  `opt1` varchar(255) DEFAULT NULL,
  `opt2` varchar(255) DEFAULT NULL,
  `opt3` varchar(255) DEFAULT NULL,
  `opt4` varchar(255) DEFAULT NULL,
  `opt5` varchar(255) DEFAULT NULL,
  `correct_answer` enum('opt1','opt2','opt3','opt4','opt5') DEFAULT NULL,
  `solution` text,
  `sumberfile` varchar(500) DEFAULT NULL,
  `modul` varchar(500) DEFAULT NULL,
  `pelajaran` varchar(500) DEFAULT NULL,
  `tag` varchar(1000) DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `dbquizapp_soal_id_uindex` (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=608 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `soal`
--

LOCK TABLES `soal` WRITE;
/*!40000 ALTER TABLE `soal` DISABLE KEYS */;
INSERT INTO `soal` VALUES (1,'soal1','pilihan1 ','b','c','d','e','opt2','jaawaban',NULL,NULL,NULL,NULL),(2,'soal2','aaaa','bb','cc','dd','eee','opt3','jawaaaaban',NULL,NULL,NULL,NULL),(3,'soal3','a3','b3','c3','d3','e3','opt1','kunjaw',NULL,NULL,NULL,NULL),(4,'apa yang dimaksud oci','oci a','oci b','oci c','oci d','oci e','opt4','jawaban D karena bla',NULL,NULL,NULL,NULL),(5,'soal5','a5','b5','c5','d5','e5','opt4','jawaban D karena bla',NULL,NULL,NULL,NULL),(6,'apa soal 6','a6','b6','c6','d6','46','opt3','jawaban D karena bla',NULL,NULL,NULL,NULL),(7,'7 soal ini','a7','b7','c7','d7','e7','opt4','jawaban D karena bla',NULL,NULL,NULL,NULL),(8,'soal ke 8','8','8b','8c ','8d','oci e','opt2','jawaban D karena bla',NULL,NULL,NULL,NULL),(9,'soal 9 ini ','aaa9','bbb0','ccc9','dddd9','eee9','opt1','acvdsds ',NULL,NULL,NULL,NULL),(10,'What is the capital of France?','London','Berlin','Paris','Madrid','Rome','opt1','Paris is the capital and most populous city of France.',NULL,NULL,NULL,NULL),(11,'What is the capital of France?','London','Berlin','Paris','Madrid','Rome','opt1','Paris is the capital and most populous city of France.',NULL,NULL,NULL,NULL),(12,'What is the capital of France?','London','Berlin','Paris','Madrid','Rome','opt1','Paris is the capital and most populous city of France.','https://afdaliable.quickconnect.to/d/f/119Zr19rHtF979oDHkDBnLiNCMzKA7Ux','EKMA4444','akuntansi',NULL),(13,'What is the capital of France?','London','Berlin','Paris','Madrid','Rome','opt1','Paris is the capital and most populous city of France.','https://afdaliable.quickconnect.to/d/f/119Zr19rHtF979oDHkDBnLiNCMzKA7Ux','EKMA4444','akuntansi','soal modul'),(607,'<br><table class=\'quiz-table\'><thead><tr><th>Biaya Overhead Pabrik</th><th>Fungsi Biaya</th></tr></thead><tbody><tr><td>1. Listrik</td><td>Rp400 + Rp30 per jam kerja langsung</td></tr><tr><td>2. Pemeliharaan</td><td>Rp350 + Rp10 per jam kerja langsung</td></tr><tr><td>3. Gaji Supervisor</td><td>Rp28.000 per bulan</td></tr><tr><td>4. Bahan Penolong</td><td>Rp12 per jam kerja langsung</td></tr></tbody></table><br> jika produksi bulan September diperkirakan sebesar 1.500 unit, dan memerlukan jam kerja langsung sebanyak 1.000 jam, maka estimasi biaya overhead pabrik adalah sebesar ....','Rp80.750','Rp98.000','Rp106.750','Rp72.000','','opt1','Rp80.750','https://afdaliable.quickconnect.to/d/s/13noWcN3dYdQRT6CSRq02mTh9bYdrNxp/-qRg7dD9ak1xSdfz1D8VkOF5CFNthGeM-TbGAhImiXQw','EKMA4314','Akuntansi Manajemen','M2, Tes-formatif-1');
/*!40000 ALTER TABLE `soal` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `user_subscriptions`
--

DROP TABLE IF EXISTS `user_subscriptions`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `user_subscriptions` (
  `id` int NOT NULL AUTO_INCREMENT,
  `user_id` char(36) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL,
  `plan_id` int NOT NULL,
  `start_date` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `end_date` timestamp NULL DEFAULT NULL COMMENT 'NULL for lifetime subscriptions',
  `status` enum('active','expired','cancelled') DEFAULT 'active',
  `created_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `dbquizapp_user_subscriptions_id_uindex` (`id`),
  KEY `dbquizapp_user_subscriptions_user_id_idx` (`user_id`),
  KEY `dbquizapp_user_subscriptions_plan_id_idx` (`plan_id`),
  KEY `dbquizapp_user_subscriptions_status_idx` (`status`),
  CONSTRAINT `user_subscriptions_ibfk_1` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`),
  CONSTRAINT `user_subscriptions_ibfk_2` FOREIGN KEY (`plan_id`) REFERENCES `premium_plans` (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=6 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `user_subscriptions`
--

LOCK TABLES `user_subscriptions` WRITE;
/*!40000 ALTER TABLE `user_subscriptions` DISABLE KEYS */;
INSERT INTO `user_subscriptions` VALUES (1,'6a8c7e6e-ffba-4e67-a644-0657d2a1bd92',5,'2025-03-16 07:54:05','2025-05-15 07:54:05','active','2025-03-16 07:54:05','2025-03-16 07:54:05'),(2,'6a8c7e6e-ffba-4e67-a644-0657d2a1bd92',5,'2025-03-16 08:21:44','2025-05-15 08:21:45','active','2025-03-16 08:21:44','2025-03-16 08:21:44'),(3,'ba46297e-764d-4949-82d6-45623c60c325',5,'2025-03-16 08:29:26','2025-05-15 08:29:27','active','2025-03-16 08:29:26','2025-03-16 08:29:26'),(4,'ba46297e-764d-4949-82d6-45623c60c325',1,'2025-03-16 13:59:04','2025-04-15 13:59:04','active','2025-03-16 13:59:04','2025-03-16 13:59:04'),(5,'5f63d46f-122f-4b6d-a350-528414b3703c',1,'2025-03-17 04:43:15','2025-04-16 04:43:15','active','2025-03-17 04:43:15','2025-03-17 04:43:15');
/*!40000 ALTER TABLE `user_subscriptions` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `users`
--

DROP TABLE IF EXISTS `users`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `users` (
  `id` char(36) NOT NULL,
  `email` varchar(255) NOT NULL,
  `display_name` varchar(100) NOT NULL,
  `provider` varchar(50) DEFAULT 'local',
  `picture_url` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL,
  `phone_number` varchar(20) DEFAULT NULL,
  `deleted_at` timestamp NULL DEFAULT NULL,
  `created_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `updated_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE KEY `email` (`email`),
  UNIQUE KEY `dbquizapp_users_id_uindex` (`id`),
  UNIQUE KEY `dbquizapp_users_email_uindex` (`email`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `users`
--

LOCK TABLES `users` WRITE;
/*!40000 ALTER TABLE `users` DISABLE KEYS */;
INSERT INTO `users` VALUES ('5f63d46f-122f-4b6d-a350-528414b3703c','afdhal.kurniawan28@gmail.com','afdhal kurniawan','local','https://lh3.googleusercontent.com/a/ACg8ocJytbg8R2BvHGrE1gn6mcmnYOjqYSqucNeaHI9Ue2xCwxoxkw=s96-c',NULL,NULL,'2025-03-13 14:56:18','2025-03-13 14:56:18'),('6a8c7e6e-ffba-4e67-a644-0657d2a1bd92','arazakibsr@gmail.com','Afdhal Kurniawan','local','https://lh3.googleusercontent.com/a/ACg8ocKAKcYS6QFxJHfodIv8hlT4OZKtzZ12DtCnYaUDw50ZhFjdbvLFjg=s96-c','NULL',NULL,'2025-03-15 09:21:04','2025-03-16 07:21:01'),('93719f49-72b4-4f46-97a3-bc0b68cee8ed','fesstubir@gmail.com','tubir fess','local','https://lh3.googleusercontent.com/a/ACg8ocL3Z9i7FpC8tecanuXvdYSpzZIqN8nFfyFcq3UrlGXuEUFuZQ=s96-c',NULL,NULL,'2025-03-17 05:40:15','2025-03-17 05:40:15'),('ba46297e-764d-4949-82d6-45623c60c325','afdalsitp@gmail.com','afdal sitp','local','https://lh3.googleusercontent.com/a/ACg8ocKurr_Ul5hunCxVNiR0BC9l6KD1RTMl3n4tXhKvD3r6z0dS-g=s96-c',NULL,NULL,'2025-03-13 08:22:33','2025-03-13 08:22:33');
/*!40000 ALTER TABLE `users` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `users_history`
--

DROP TABLE IF EXISTS `users_history`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `users_history` (
  `id` int NOT NULL AUTO_INCREMENT,
  `username` varchar(500) NOT NULL,
  `jumlah_TO` int DEFAULT NULL,
  `nilai_TO` decimal(5,2) DEFAULT NULL,
  `tanggal_TO` date DEFAULT NULL,
  `kategori` varchar(500) DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `username` (`username`),
  UNIQUE KEY `dbquizapp_users_history_id_uindex` (`id`),
  UNIQUE KEY `dbquizapp_users_history_username_uindex` (`username`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `users_history`
--

LOCK TABLES `users_history` WRITE;
/*!40000 ALTER TABLE `users_history` DISABLE KEYS */;
/*!40000 ALTER TABLE `users_history` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Table structure for table `verification`
--

DROP TABLE IF EXISTS `verification`;
/*!40101 SET @saved_cs_client     = @@character_set_client */;
/*!50503 SET character_set_client = utf8mb4 */;
CREATE TABLE `verification` (
  `id` varchar(255) NOT NULL,
  `identifier` varchar(255) NOT NULL,
  `value` varchar(255) NOT NULL,
  `expiresAt` datetime NOT NULL,
  `createdAt` datetime NOT NULL,
  `updatedAt` datetime NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
/*!40101 SET character_set_client = @saved_cs_client */;

--
-- Dumping data for table `verification`
--

LOCK TABLES `verification` WRITE;
/*!40000 ALTER TABLE `verification` DISABLE KEYS */;
/*!40000 ALTER TABLE `verification` ENABLE KEYS */;
UNLOCK TABLES;

--
-- Dumping routines for database 'dbquizapp'
--
--
-- WARNING: can't read the INFORMATION_SCHEMA.libraries table. It's most probably an old server 8.0.41-0ubuntu0.24.04.1.
--
/*!40103 SET TIME_ZONE=@OLD_TIME_ZONE */;

/*!40101 SET SQL_MODE=@OLD_SQL_MODE */;
/*!40014 SET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS */;
/*!40014 SET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS */;
/*!40101 SET CHARACTER_SET_CLIENT=@OLD_CHARACTER_SET_CLIENT */;
/*!40101 SET CHARACTER_SET_RESULTS=@OLD_CHARACTER_SET_RESULTS */;
/*!40101 SET COLLATION_CONNECTION=@OLD_COLLATION_CONNECTION */;
/*!40111 SET SQL_NOTES=@OLD_SQL_NOTES */;

-- Dump completed on 2025-08-08  9:02:26
