CREATE SCHEMA IF NOT EXISTS dbquizapp;

CREATE TABLE IF NOT EXISTS dbquizapp.harga_paket (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  id_paket_soal int NOT NULL,
  koin int NOT NULL DEFAULT 0,
  harga decimal(10, 2) NOT NULL DEFAULT 0.00,
  is_free tinyint(1) NOT NULL DEFAULT 0,
  created_at timestamp DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamp DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX dbquizapp_harga_paket_id_uindex ON dbquizapp.harga_paket (id);
CREATE INDEX dbquizapp_harga_paket_id_paket_soal_idx ON dbquizapp.harga_paket (id_paket_soal);

CREATE TABLE IF NOT EXISTS dbquizapp.jawaban_user (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  soal_id int DEFAULT NULL,
  user_id int DEFAULT NULL,
  jawaban varchar(500) DEFAULT NULL,
  is_correct tinyint(1) DEFAULT NULL
);

CREATE UNIQUE INDEX dbquizapp_jawaban_user_id_uindex ON dbquizapp.jawaban_user (id);
CREATE INDEX dbquizapp_jawaban_user_soal_id_idx ON dbquizapp.jawaban_user (soal_id);
CREATE INDEX dbquizapp_jawaban_user_user_id_idx ON dbquizapp.jawaban_user (user_id);

CREATE TABLE IF NOT EXISTS dbquizapp.kategori_soal (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  nama_kategori varchar(500) NOT NULL UNIQUE
);

CREATE UNIQUE INDEX dbquizapp_kategori_soal_id_uindex ON dbquizapp.kategori_soal (id);
CREATE UNIQUE INDEX dbquizapp_kategori_soal_nama_kategori_uindex ON dbquizapp.kategori_soal (nama_kategori);

CREATE TABLE IF NOT EXISTS dbquizapp.paket_soal (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  nama_paket_soal varchar(500) NOT NULL,
  kategori_id int DEFAULT NULL
);

CREATE UNIQUE INDEX dbquizapp_paket_soal_id_uindex ON dbquizapp.paket_soal (id);
CREATE INDEX dbquizapp_paket_soal_kategori_id_idx ON dbquizapp.paket_soal (kategori_id);

CREATE TABLE IF NOT EXISTS dbquizapp.paket_soal_items (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  paket_soal_id int DEFAULT NULL,
  soal_id int DEFAULT NULL
);

CREATE UNIQUE INDEX dbquizapp_paket_soal_items_id_uindex ON dbquizapp.paket_soal_items (id);
CREATE INDEX dbquizapp_paket_soal_items_paket_soal_id_idx ON dbquizapp.paket_soal_items (paket_soal_id);
CREATE INDEX dbquizapp_paket_soal_items_soal_id_idx ON dbquizapp.paket_soal_items (soal_id);

CREATE TABLE IF NOT EXISTS dbquizapp.quiz_answers (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  session_id varchar(500) NOT NULL,
  soal_id int NOT NULL,
  selected_option int NOT NULL,
  is_correct tinyint(1) DEFAULT NULL
);

CREATE UNIQUE INDEX dbquizapp_quiz_answers_id_uindex ON dbquizapp.quiz_answers (id);
CREATE INDEX dbquizapp_quiz_answers_soal_id_idx ON dbquizapp.quiz_answers (soal_id);
CREATE INDEX dbquizapp_quiz_answers_session_id_soal_id_idx ON dbquizapp.quiz_answers (session_id, soal_id);

CREATE TABLE IF NOT EXISTS dbquizapp.quiz_sessions (
  id varchar(500) NOT NULL PRIMARY KEY,
  user_id varchar(500) NOT NULL,
  paket_soal_id int NOT NULL,
  start_time timestamp DEFAULT CURRENT_TIMESTAMP,
  duration int NOT NULL,
  status enum('ongoing', 'completed') DEFAULT 'ongoing'
);

CREATE UNIQUE INDEX dbquizapp_quiz_sessions_id_uindex ON dbquizapp.quiz_sessions (id);
CREATE INDEX dbquizapp_quiz_sessions_paket_soal_id_idx ON dbquizapp.quiz_sessions (paket_soal_id);
CREATE INDEX dbquizapp_quiz_sessions_user_id_status_idx ON dbquizapp.quiz_sessions (user_id, status);

CREATE TABLE IF NOT EXISTS dbquizapp.roles (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  name varchar(500) NOT NULL UNIQUE
);

CREATE UNIQUE INDEX dbquizapp_roles_id_uindex ON dbquizapp.roles (id);
CREATE UNIQUE INDEX dbquizapp_roles_name_uindex ON dbquizapp.roles (name);

CREATE TABLE IF NOT EXISTS dbquizapp.sessions (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  user_id char NOT NULL,
  token varchar(500) NOT NULL UNIQUE,
  expires_at timestamp NOT NULL,
  ip_address varchar(500) DEFAULT NULL,
  user_agent text DEFAULT NULL,
  created_at timestamp DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamp DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX dbquizapp_sessions_id_uindex ON dbquizapp.sessions (id);
CREATE UNIQUE INDEX dbquizapp_sessions_token_uindex ON dbquizapp.sessions (token);
CREATE INDEX dbquizapp_sessions_user_id_idx ON dbquizapp.sessions (user_id);

CREATE TABLE IF NOT EXISTS dbquizapp.soal (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  soal text NOT NULL,
  opt1 varchar(500) DEFAULT NULL,
  opt2 varchar(500) DEFAULT NULL,
  opt3 varchar(500) DEFAULT NULL,
  opt4 varchar(500) DEFAULT NULL,
  opt5 varchar(500) DEFAULT NULL,
  correct_answer enum('option1', 'option2', 'option3', 'option4', 'option5') DEFAULT NULL,
  solution text DEFAULT NULL,
  sumberfile varchar(500) DEFAULT NULL,
  `Modul` varchar(500) DEFAULT NULL,
  `Pelajaran` varchar(500) DEFAULT NULL
);

CREATE UNIQUE INDEX dbquizapp_soal_id_uindex ON dbquizapp.soal (id);

CREATE TABLE IF NOT EXISTS dbquizapp.users (
  id char NOT NULL PRIMARY KEY,
  email varchar(500) NOT NULL UNIQUE,
  display_name varchar(500) NOT NULL,
  provider varchar(500) DEFAULT NULL,
  picture_url varchar(500) DEFAULT NULL,
  deleted_at timestamp DEFAULT NULL,
  created_at timestamp DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamp DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX dbquizapp_users_id_uindex ON dbquizapp.users (id);
CREATE UNIQUE INDEX dbquizapp_users_email_uindex ON dbquizapp.users (email);

CREATE TABLE IF NOT EXISTS dbquizapp.users_history (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  username varchar(500) NOT NULL UNIQUE,
  jumlah_TO int DEFAULT NULL,
  nilai_TO decimal(5, 2) DEFAULT NULL,
  tanggal_TO date DEFAULT NULL,
  kategori varchar(500) DEFAULT NULL
);

CREATE UNIQUE INDEX dbquizapp_users_history_id_uindex ON dbquizapp.users_history (id);
CREATE UNIQUE INDEX dbquizapp_users_history_username_uindex ON dbquizapp.users_history (username);

-- Premium Plan Tables
CREATE TABLE IF NOT EXISTS dbquizapp.premium_plans (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  name varchar(100) NOT NULL,
  description text NOT NULL,
  price double(10, 2) NOT NULL,
  duration_days int NOT NULL COMMENT 'Duration in days, 0 for lifetime',
  is_lifetime tinyint(1) NOT NULL DEFAULT 0,
  features text NOT NULL COMMENT 'JSON array of features',
  created_at timestamp DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamp DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX dbquizapp_premium_plans_id_uindex ON dbquizapp.premium_plans (id);
CREATE UNIQUE INDEX dbquizapp_premium_plans_name_uindex ON dbquizapp.premium_plans (name);

-- User Subscriptions Table
CREATE TABLE IF NOT EXISTS dbquizapp.user_subscriptions (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  user_id char NOT NULL,
  plan_id int NOT NULL,
  start_date timestamp DEFAULT CURRENT_TIMESTAMP,
  end_date timestamp NULL COMMENT 'NULL for lifetime subscriptions',
  status enum('active', 'expired', 'cancelled') DEFAULT 'active',
  created_at timestamp DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamp DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX dbquizapp_user_subscriptions_id_uindex ON dbquizapp.user_subscriptions (id);
CREATE INDEX dbquizapp_user_subscriptions_user_id_idx ON dbquizapp.user_subscriptions (user_id);
CREATE INDEX dbquizapp_user_subscriptions_plan_id_idx ON dbquizapp.user_subscriptions (plan_id);
CREATE INDEX dbquizapp_user_subscriptions_status_idx ON dbquizapp.user_subscriptions (status);

-- Premium Quiz Access Table
CREATE TABLE IF NOT EXISTS dbquizapp.premium_quiz_access (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  paket_soal_id int NOT NULL,
  min_plan_id int NOT NULL COMMENT 'Minimum plan ID required to access this quiz',
  created_at timestamp DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamp DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX dbquizapp_premium_quiz_access_id_uindex ON dbquizapp.premium_quiz_access (id);
CREATE UNIQUE INDEX dbquizapp_premium_quiz_access_paket_soal_id_uindex ON dbquizapp.premium_quiz_access (paket_soal_id);
CREATE INDEX dbquizapp_premium_quiz_access_min_plan_id_idx ON dbquizapp.premium_quiz_access (min_plan_id);

-- Payment Transactions Table
CREATE TABLE IF NOT EXISTS dbquizapp.payment_transactions (
  id int NOT NULL AUTO_INCREMENT PRIMARY KEY,
  user_id char NOT NULL,
  plan_id int NOT NULL,
  amount decimal(10, 2) NOT NULL,
  transaction_id varchar(255) NOT NULL COMMENT 'Transaction ID from payment gateway',
  payment_link varchar(255) NOT NULL,
  status enum('pending', 'completed', 'failed', 'expired') DEFAULT 'pending',
  payment_method varchar(100) NULL,
  payment_details text NULL COMMENT 'JSON with payment details',
  webhook_data text NULL COMMENT 'JSON with webhook data',
  created_at timestamp DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamp DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX dbquizapp_payment_transactions_id_uindex ON dbquizapp.payment_transactions (id);
CREATE UNIQUE INDEX dbquizapp_payment_transactions_transaction_id_uindex ON dbquizapp.payment_transactions (transaction_id);
CREATE INDEX dbquizapp_payment_transactions_user_id_idx ON dbquizapp.payment_transactions (user_id);
CREATE INDEX dbquizapp_payment_transactions_plan_id_idx ON dbquizapp.payment_transactions (plan_id);
CREATE INDEX dbquizapp_payment_transactions_status_idx ON dbquizapp.payment_transactions (status);

ALTER TABLE dbquizapp.harga_paket ADD CONSTRAINT harga_paket_ibfk_1 FOREIGN KEY (id_paket_soal) REFERENCES dbquizapp.paket_soal (id);
ALTER TABLE dbquizapp.jawaban_user ADD CONSTRAINT jawaban_user_ibfk_1 FOREIGN KEY (soal_id) REFERENCES dbquizapp.soal (id);
ALTER TABLE dbquizapp.jawaban_user ADD CONSTRAINT jawaban_user_ibfk_2 FOREIGN KEY (user_id) REFERENCES dbquizapp.users_history (id);
ALTER TABLE dbquizapp.paket_soal ADD CONSTRAINT paket_soal_ibfk_1 FOREIGN KEY (kategori_id) REFERENCES dbquizapp.kategori_soal (id);
ALTER TABLE dbquizapp.paket_soal_items ADD CONSTRAINT paket_soal_items_ibfk_1 FOREIGN KEY (paket_soal_id) REFERENCES dbquizapp.paket_soal (id);
ALTER TABLE dbquizapp.paket_soal_items ADD CONSTRAINT paket_soal_items_ibfk_2 FOREIGN KEY (soal_id) REFERENCES dbquizapp.soal (id);
ALTER TABLE dbquizapp.quiz_answers ADD CONSTRAINT quiz_answers_ibfk_1 FOREIGN KEY (session_id) REFERENCES dbquizapp.quiz_sessions (id);
ALTER TABLE dbquizapp.quiz_answers ADD CONSTRAINT quiz_answers_ibfk_2 FOREIGN KEY (soal_id) REFERENCES dbquizapp.soal (id);
ALTER TABLE dbquizapp.quiz_sessions ADD CONSTRAINT quiz_sessions_ibfk_1 FOREIGN KEY (paket_soal_id) REFERENCES dbquizapp.paket_soal (id);
ALTER TABLE dbquizapp.sessions ADD CONSTRAINT sessions_ibfk_1 FOREIGN KEY (user_id) REFERENCES dbquizapp.users (id);

-- Add Foreign Key Constraints
ALTER TABLE dbquizapp.user_subscriptions ADD CONSTRAINT user_subscriptions_ibfk_1 FOREIGN KEY (user_id) REFERENCES dbquizapp.users (id);
ALTER TABLE dbquizapp.user_subscriptions ADD CONSTRAINT user_subscriptions_ibfk_2 FOREIGN KEY (plan_id) REFERENCES dbquizapp.premium_plans (id);

ALTER TABLE dbquizapp.premium_quiz_access ADD CONSTRAINT premium_quiz_access_ibfk_1 FOREIGN KEY (paket_soal_id) REFERENCES dbquizapp.paket_soal (id);
ALTER TABLE dbquizapp.premium_quiz_access ADD CONSTRAINT premium_quiz_access_ibfk_2 FOREIGN KEY (min_plan_id) REFERENCES dbquizapp.premium_plans (id);

ALTER TABLE dbquizapp.payment_transactions ADD CONSTRAINT payment_transactions_ibfk_1 FOREIGN KEY (user_id) REFERENCES dbquizapp.users (id);
ALTER TABLE dbquizapp.payment_transactions ADD CONSTRAINT payment_transactions_ibfk_2 FOREIGN KEY (plan_id) REFERENCES dbquizapp.premium_plans (id);

-- Insert default premium plans
INSERT INTO dbquizapp.premium_plans (name, description, price, duration_days, is_lifetime, features)
VALUES 
('Silver Plan', 'Basic premium features with limited access', 99000.00, 30, 0, '["Access to basic premium quizzes", "Priority support", "Ad-free experience"]'),
('Gold Plan', 'Advanced premium features with extended access', 199000.00, 90, 0, '["Access to all premium quizzes", "Priority support", "Ad-free experience", "Detailed performance analytics", "Downloadable quiz reports"]'),
('Platinum Plan', 'Complete premium package with all features', 299000.00, 180, 0, '["Access to all premium quizzes", "Priority support", "Ad-free experience", "Detailed performance analytics", "Downloadable quiz reports", "Personalized learning path", "Expert consultation"]'),
('Ultimate Plan', 'Lifetime access to all premium features', 999000.00, 0, 1, '["Lifetime access to all premium quizzes", "Priority support", "Ad-free experience", "Detailed performance analytics", "Downloadable quiz reports", "Personalized learning path", "Expert consultation", "Early access to new features"]');