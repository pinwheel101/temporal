use std::fs;
use std::io;
use brotli::enc::BrotliEncoderParams;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce
};
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::{SaltString, PasswordHash};
use rand::RngCore;

// Base94 문자셋 (ASCII 33-126)
const BASE94_CHARSET: &[u8] = 
    b"!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";

// 패스워드에서 32바이트 암호화 키 생성
fn derive_key_from_password(password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    use argon2::Algorithm;
    
    let argon2 = Argon2::new(
        Algorithm::Argon2id,  // 가장 안전한 알고리즘
        argon2::Version::V0x13,
        argon2::Params::default()
    );
    
    let mut key = [0u8; 32];
    argon2.hash_password_into(
        password.as_bytes(),
        salt,
        &mut key
    ).map_err(|e| format!("키 생성 실패: {}", e))?;
    
    Ok(key)
}

// Base94 인코딩
fn base94_encode(data: &[u8]) -> String {
    use num_bigint::BigUint;
    
    let mut result = String::new();
    let mut num = BigUint::from_bytes_be(data);
    let base = BigUint::from(94u32);
    let zero = BigUint::from(0u32);
    
    if num == zero {
        return String::from(char::from(BASE94_CHARSET[0]));
    }
    
    while num > zero {
        let remainder = &num % &base;
        let idx = remainder.to_u32_digits()[0] as usize;
        result.push(char::from(BASE94_CHARSET[idx]));
        num /= &base;
    }
    
    result.chars().rev().collect()
}

// Base94 디코딩
fn base94_decode(encoded: &str) -> Result<Vec<u8>, String> {
    use num_bigint::BigUint;
    
    let mut num = BigUint::from(0u32);
    let base = BigUint::from(94u32);
    
    for ch in encoded.chars() {
        let pos = BASE94_CHARSET.iter()
            .position(|&c| c == ch as u8)
            .ok_or_else(|| format!("잘못된 문자: {}", ch))?;
        num = num * &base + pos;
    }
    
    Ok(num.to_bytes_be())
}

// Brotli 압축
fn compress_data(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let params = BrotliEncoderParams {
        quality: 11,
        ..Default::default()
    };
    
    brotli::BrotliCompress(
        &mut &data[..],
        &mut output,
        &params
    )?;
    
    Ok(output)
}

// Brotli 압축 해제
fn decompress_data(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    brotli::BrotliDecompress(
        &mut &data[..],
        &mut output
    )?;
    Ok(output)
}

// 암호화 메인 함수
pub fn encrypt_and_encode(
    input_path: &str,
    password: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // 1. 파일 읽기
    let plaintext = fs::read_to_string(input_path)?;
    println!("✓ 파일 읽기 완료: {} bytes", plaintext.len());
    
    // 2. Brotli 압축
    let compressed = compress_data(plaintext.as_bytes())?;
    println!("✓ 압축 완료: {} bytes -> {} bytes ({:.1}% 감소)",
        plaintext.len(), compressed.len(),
        (1.0 - compressed.len() as f64 / plaintext.len() as f64) * 100.0
    );
    
    // 3. Salt 생성 (암호화마다 다른 salt 사용)
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    
    // 4. 패스워드에서 키 생성
    let key = derive_key_from_password(password, &salt)?;
    println!("✓ 암호화 키 생성 완료");
    
    // 5. Nonce 생성 (24바이트)
    let mut nonce_bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);
    
    // 6. XChaCha20-Poly1305 암호화
    let cipher = XChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| format!("Cipher 생성 실패: {}", e))?;
    
    let ciphertext = cipher.encrypt(nonce, compressed.as_ref())
        .map_err(|e| format!("암호화 실패: {}", e))?;
    
    println!("✓ 암호화 완료: {} bytes", ciphertext.len());
    
    // 7. Base94 인코딩
    // 형식: [salt(16) | nonce(24) | ciphertext]
    let mut combined = Vec::new();
    combined.extend_from_slice(&salt);
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    
    let encoded = base94_encode(&combined);
    println!("✓ Base94 인코딩 완료: {} chars", encoded.len());
    
    Ok(encoded)
}

// 복호화 메인 함수
pub fn decode_and_decrypt(
    encoded: &str,
    password: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("복호화 시작...");
    
    // 1. Base94 디코딩
    let decoded = base94_decode(encoded)?;
    println!("✓ Base94 디코딩 완료: {} bytes", decoded.len());
    
    // 2. Salt, Nonce, Ciphertext 분리
    if decoded.len() < 40 {  // 최소: salt(16) + nonce(24)
        return Err("데이터가 너무 짧습니다".into());
    }
    
    let salt = &decoded[0..16];
    let nonce_bytes = &decoded[16..40];
    let ciphertext = &decoded[40..];
    
    // 3. 패스워드에서 키 재생성
    let key = derive_key_from_password(password, salt)?;
    println!("✓ 암호화 키 재생성 완료");
    
    // 4. 복호화
    let cipher = XChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| format!("Cipher 생성 실패: {}", e))?;
    
    let nonce = XNonce::from_slice(nonce_bytes);
    let decrypted = cipher.decrypt(nonce, ciphertext)
        .map_err(|_| "복호화 실패: 잘못된 패스워드 또는 손상된 데이터")?;
    
    println!("✓ 복호화 완료: {} bytes", decrypted.len());
    
    // 5. Brotli 압축 해제
    let decompressed = decompress_data(&decrypted)?;
    println!("✓ 압축 해제 완료: {} bytes", decompressed.len());
    
    // 6. 문자열로 변환
    let plaintext = String::from_utf8(decompressed)
        .map_err(|_| "UTF-8 변환 실패")?;
    
    Ok(plaintext)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 고정 패스워드 사용
    let password = "my_super_secret_password_2024";
    
    println!("=== 암호화 시작 ===");
    let encrypted = encrypt_and_encode("input.txt", password)?;
    println!("\n암호화 결과:\n{}\n", encrypted);
    
    // 결과를 파일로 저장
    fs::write("encrypted.txt", &encrypted)?;
    println!("✓ 암호화 결과 저장: encrypted.txt\n");
    
    println!("=== 복호화 시작 ===");
    let decrypted = decode_and_decrypt(&encrypted, password)?;
    println!("\n복호화 결과:\n{}\n", decrypted);
    
    // 복호화 결과 저장
    fs::write("decrypted.txt", &decrypted)?;
    println!("✓ 복호화 결과 저장: decrypted.txt");
    
    Ok(())
}
