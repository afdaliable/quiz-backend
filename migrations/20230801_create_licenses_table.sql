-- Create licenses table
CREATE TABLE IF NOT EXISTS licenses (
    id INT AUTO_INCREMENT PRIMARY KEY,
    license_code VARCHAR(255) NOT NULL,
    user_id VARCHAR(255) NOT NULL,
    plan_id INT NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'Active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expired_at TIMESTAMP NULL,
    is_lifetime BOOLEAN DEFAULT FALSE,
    product_id VARCHAR(255) NOT NULL,
    UNIQUE KEY (license_code),
    INDEX (user_id),
    INDEX (plan_id)
);

-- Add foreign key constraints
ALTER TABLE licenses
ADD CONSTRAINT fk_licenses_user_id
FOREIGN KEY (user_id) REFERENCES users(id)
ON DELETE CASCADE;

ALTER TABLE licenses
ADD CONSTRAINT fk_licenses_plan_id
FOREIGN KEY (plan_id) REFERENCES premium_plans(id)
ON DELETE RESTRICT; 