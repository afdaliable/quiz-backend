CREATE TABLE users (
    id CHAR(36) PRIMARY KEY, -- Menyimpan UUID dalam format teks
    email VARCHAR(255) UNIQUE NOT NULL, -- Email sebaiknya VARCHAR dengan UNIQUE constraint
    display_name VARCHAR(100) NOT NULL, -- Nama pengguna
    deleted_at TIMESTAMP NULL DEFAULT NULL, -- Untuk soft delete, NULL jika belum dihapus
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP, -- Timestamp saat dibuat
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP -- Timestamp saat diperbarui
);
