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