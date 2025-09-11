# CORS Configuration Fix

## Problem
CORS error terjadi karena backend menggunakan `.allow_any_origin()` dengan `.supports_credentials()`, yang tidak kompatibel dengan browser security policy.

## Solution
Backend sekarang menggunakan daftar domain yang spesifik yang diizinkan akses, dan mendukung konfigurasi via environment variable.

## Environment Variable

Tambahkan di environment backend:

```bash
CORS_ALLOWED_ORIGINS="http://localhost:3088,https://kuis-admin.canducation.com,https://your-production-domain.com"
```

## Default Configuration
Jika tidak ada environment variable, backend akan menggunakan:
- `http://localhost:3088` (development)
- `https://kuis-admin.canducation.com` (production)

## How to Update

1. **Development**: Tidak perlu perubahan, localhost sudah included
2. **Production**: Set environment variable `CORS_ALLOWED_ORIGINS` dengan domain frontend production Anda

## Verification

Setelah deploy backend dengan perubahan ini, check log backend untuk memastikan origin yang benar telah di-add:

```
CORS: Allowing origin: http://localhost:3088
CORS: Allowing origin: https://kuis-admin.canducation.com
```

## Key Changes Made

1. Removed `.allow_any_origin()`
2. Added specific `.allowed_origin()` calls
3. Made origins configurable via environment variable
4. Added logging untuk debug CORS configuration

## Security
Approach ini lebih aman karena:
- Hanya domain yang eksplisit diizinkan yang bisa mengakses API
- Credentials masih supported untuk domain yang diizinkan
- Configurable per environment (dev/prod)