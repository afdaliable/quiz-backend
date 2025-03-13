CREATE TABLE users (
    id CHAR(36) PRIMARY KEY, -- Menyimpan UUID dalam format teks
    email VARCHAR(255) UNIQUE NOT NULL, -- Email sebaiknya VARCHAR dengan UNIQUE constraint
    display_name VARCHAR(100) NOT NULL, -- Nama pengguna
    deleted_at TIMESTAMP NULL DEFAULT NULL, -- Untuk soft delete, NULL jika belum dihapus
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP, -- Timestamp saat dibuat
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP -- Timestamp saat diperbarui
);

CREATE TABLE harga_paket (
    id INT AUTO_INCREMENT PRIMARY KEY,
    id_paket_soal INT NOT NULL,
    koin INT NOT NULL DEFAULT 0,
    harga DECIMAL(10, 2) NOT NULL DEFAULT 0.00,
    is_free BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (id_paket_soal) REFERENCES paket_soal(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS sessions (
  id BIGINT AUTO_INCREMENT PRIMARY KEY,
  user_id CHAR(36) NOT NULL,
  token VARCHAR(255) NOT NULL UNIQUE,
  expires_at TIMESTAMP NOT NULL,
  ip_address VARCHAR(45),
  user_agent TEXT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_sessions_token ON sessions(token);
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);