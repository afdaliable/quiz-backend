-- Add new fields to premium_plans table for Mayar integration
ALTER TABLE dbquizapp.premium_plans 
ADD COLUMN mayar_product_id VARCHAR(255) NULL COMMENT 'Product ID from Mayar',
ADD COLUMN mayar_link_payment VARCHAR(255) NULL COMMENT 'Link to Mayar payment page';

-- Create license_codes table to store license codes from Mayar
CREATE TABLE IF NOT EXISTS dbquizapp.license_codes (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  license_code VARCHAR(255) NOT NULL UNIQUE COMMENT 'License code from Mayar',
  user_id char NULL COMMENT 'User ID who activated this license',
  plan_id int NOT NULL COMMENT 'Premium plan ID',
  status enum('active', 'expired', 'cancelled') DEFAULT 'active',
  transaction_id VARCHAR(255) NULL COMMENT 'Transaction ID from Mayar',
  product_id VARCHAR(255) NOT NULL COMMENT 'Product ID from Mayar',
  customer_id VARCHAR(255) NULL COMMENT 'Customer ID from Mayar',
  customer_name VARCHAR(255) NULL COMMENT 'Customer name from Mayar',
  customer_email VARCHAR(255) NULL COMMENT 'Customer email from Mayar',
  expired_at TIMESTAMP NULL COMMENT 'Expiration date from Mayar',
  activation_limit VARCHAR(255) NULL COMMENT 'Activation limit from Mayar',
  use_count INT NULL COMMENT 'Use count from Mayar',
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  license_data TEXT NULL COMMENT 'Full JSON response from Mayar license verification'
);

CREATE UNIQUE INDEX dbquizapp_license_codes_id_uindex ON dbquizapp.license_codes (id);
CREATE UNIQUE INDEX dbquizapp_license_codes_license_code_uindex ON dbquizapp.license_codes (license_code);
CREATE INDEX dbquizapp_license_codes_user_id_idx ON dbquizapp.license_codes (user_id);
CREATE INDEX dbquizapp_license_codes_plan_id_idx ON dbquizapp.license_codes (plan_id);
CREATE INDEX dbquizapp_license_codes_status_idx ON dbquizapp.license_codes (status);

-- Add foreign key constraints
ALTER TABLE dbquizapp.license_codes ADD CONSTRAINT license_codes_ibfk_1 FOREIGN KEY (user_id) REFERENCES dbquizapp.users (id);
ALTER TABLE dbquizapp.license_codes ADD CONSTRAINT license_codes_ibfk_2 FOREIGN KEY (plan_id) REFERENCES dbquizapp.premium_plans (id);

-- Update config for Mayar SaaS API
-- Note: This is just a comment, you'll need to update the config.json file manually
-- Add "mayar_saas_api_url": "https://api.mayar.id/saas/v1" to the payment section in config.json 