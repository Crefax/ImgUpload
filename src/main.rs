use actix_web::{web, App, HttpServer, HttpResponse, Error};
use actix_multipart::Multipart;
use actix_files::NamedFile;
use futures::{StreamExt, TryStreamExt};
use rand::Rng;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use mime_guess;
use serde_json;
use serde::{Deserialize, Serialize};
use bytes::Bytes;
use log::{info, error, debug};

const UPLOAD_DIR: &str = "uploads";
const CHUNK_DIR: &str = "chunks";
const URL_LENGTH: usize = 5;
const MAX_FILE_SIZE: usize = 500_000_000; // 500 MB
const SERVER_URL: &str = "https://img.crefax.net"; // Domain adresinizi buraya yazın

#[derive(Debug, Deserialize)]
struct ChunkUploadRequest {
    chunk_number: usize,
    total_chunks: usize,
    file_id: String,
    file_name: String,
}

#[derive(Debug, Serialize)]
struct ChunkUploadResponse {
    success: bool,
    message: String,
    file_id: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    success: bool,
    message: String,
    error_code: String,
}

fn generate_random_string() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..URL_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

// Güvenli dosya yolu doğrulama fonksiyonu
fn is_safe_path(path: &Path) -> bool {
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    
    let upload_dir = match Path::new(UPLOAD_DIR).canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    
    if !canonical_path.starts_with(upload_dir) {
        return false;
    }
    
    // Sadece belirli dosya uzantılarına izin ver
    if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
        matches!(extension.to_lowercase().as_str(), 
            // Resim formatları
            "jpg" | "jpeg" | "png" | "gif" | "webp" |
            // Video formatları
            "mp4" | "webm" | "avi" | "mov" | "mkv" |
            // Ses formatları
            "mp3" | "wav" | "ogg" | "m4a"
        )
    } else {
        false
    }
}

async fn index() -> Result<NamedFile, Error> {
    Ok(NamedFile::open("templates/index.html")?)
}

async fn upload(mut payload: Multipart) -> Result<HttpResponse, Error> {
    if !Path::new(UPLOAD_DIR).exists() {
        fs::create_dir(UPLOAD_DIR)?;
    }

    let mut title = String::new();
    let mut file_info: Option<(String, String, PathBuf)> = None;

    while let Some(mut field) = payload.try_next().await? {
        let content_disposition = field.content_disposition().clone();
        
        if let Some(name) = content_disposition.get_name() {
            match name {
                "file" => {
                    if let Some(filename) = content_disposition.get_filename() {
                        let random_id = generate_random_string();
                        let file_ext = Path::new(filename)
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("jpg");
                        
                        // Dosya uzantısı kontrolü
                        if !matches!(file_ext.to_lowercase().as_str(), 
                            // Resim formatları
                            "jpg" | "jpeg" | "png" | "gif" | "webp" |
                            // Video formatları
                            "mp4" | "webm" | "avi" | "mov" | "mkv" |
                            // Ses formatları
                            "mp3" | "wav" | "ogg" | "m4a"
                        ) {
                            return Ok(HttpResponse::BadRequest()
                                .content_type("text/html; charset=utf-8")
                                .body("Desteklenmeyen dosya formatı. Lütfen geçerli bir medya dosyası seçin."));
                        }

                        let filepath = PathBuf::from(format!("{}/{}.{}", UPLOAD_DIR, random_id, file_ext));
                        let filepath_clone = filepath.clone();
                        let mut f = web::block(move || std::fs::File::create(&filepath_clone))
                            .await
                            .unwrap()?;

                        // Dosyayı chunk'lar halinde yaz
                        let mut total_bytes = 0;
                        while let Some(chunk) = field.next().await {
                            let data = chunk?;
                            total_bytes += data.len();
                            
                            // Boyut kontrolü
                            if total_bytes > MAX_FILE_SIZE {
                                // Dosyayı sil ve hata döndür
                                let filepath_clone2 = filepath.clone();
                                let _ = web::block(move || std::fs::remove_file(&filepath_clone2)).await;
                                return Ok(HttpResponse::BadRequest()
                                    .content_type("text/html; charset=utf-8")
                                    .body("Dosya boyutu çok büyük."));
                            }
                            
                            f = web::block(move || f.write_all(&data).map(|_| f)).await?.unwrap();
                        }

                        if total_bytes > 0 {
                            file_info = Some((random_id, file_ext.to_string(), filepath));
                        }
                    }
                },
                "title" => {
                    let mut buffer = Vec::new();
                    while let Some(chunk) = field.next().await {
                        buffer.extend_from_slice(&chunk?);
                    }
                    title = String::from_utf8_lossy(&buffer).to_string();
                },
                _ => {}
            }
        }
    }

    if let Some((random_id, file_ext, _)) = file_info {
        let success_template = fs::read_to_string("templates/success.html")?;
        let file_url = format!("{}.{}", random_id, file_ext);
        
        // Video dosyaları için preview URL'i, diğerleri için doğrudan dosya URL'i kullan
        let final_url = match file_ext.to_lowercase().as_str() {
            "mp4" | "webm" | "mov" => format!("/p/{}", file_url),
            _ => format!("/i/{}", file_url)
        };

        // Dosya türüne göre önizleme HTML'i oluştur
        let preview_html = match file_ext.to_lowercase().as_str() {
            // Video formatları için
            "mp4" | "webm" | "mov" => format!(
                r#"<video width="100%" controls preload="metadata">
                    <source src="/i/{}" type="video/{}" />
                    Tarayıcınız video etiketini desteklemiyor.
                </video>"#,
                file_url,
                if file_ext == "mov" { "mp4" } else { &file_ext }
            ),
            // Ses formatları için
            "mp3" | "wav" | "ogg" | "m4a" => format!(
                r#"<audio controls preload="metadata">
                    <source src="/i/{}" type="audio/{}" />
                    Tarayıcınız ses etiketini desteklemiyor.
                </audio>"#,
                file_url,
                if file_ext == "m4a" { "mp4" } else { &file_ext }
            ),
            // Resim formatları için
            _ => format!(r#"<img src="/i/{}" alt="Yüklenen resim" style="max-width: 100%;" loading="lazy">"#, file_url)
        };
        
        let response_html = success_template
            .replace("{PREVIEW}", &preview_html)
            .replace("{FILE_URL}", &final_url);
        
        // Video başlığını metadata.json dosyasına kaydet
        if matches!(file_ext.to_lowercase().as_str(), "mp4" | "webm" | "mov") && !title.is_empty() {
            let metadata_path = format!("{}/{}.json", UPLOAD_DIR, random_id);
            let metadata = format!(r#"{{"title": "{}", "filename": "{}.{}"}}"#, title, random_id, file_ext);
            fs::write(metadata_path, metadata)?;
        }

        return Ok(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(response_html));
    }

    Ok(HttpResponse::BadRequest().content_type("text/html; charset=utf-8").body("Lütfen bir medya dosyası seçin ve tekrar deneyin."))
}

async fn upload_chunk(
    info: web::Query<ChunkUploadRequest>,
    body: Bytes,
) -> Result<HttpResponse, Error> {
    info!("Chunk yükleme isteği alındı: {:?}", info);
    debug!("Chunk boyutu: {} bytes", body.len());

    if !Path::new(CHUNK_DIR).exists() {
        info!("Chunk dizini oluşturuluyor: {}", CHUNK_DIR);
        if let Err(e) = fs::create_dir(CHUNK_DIR) {
            error!("Chunk dizini oluşturulamadı: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                message: "Sunucu hatası".to_string(),
                error_code: "SERVER_ERROR".to_string(),
            }));
        }
    }

    let chunk_dir = PathBuf::from(CHUNK_DIR).join(&info.file_id);
    if !chunk_dir.exists() {
        info!("Dosya için chunk dizini oluşturuluyor: {:?}", chunk_dir);
        if let Err(e) = fs::create_dir(&chunk_dir) {
            error!("Dosya chunk dizini oluşturulamadı: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                message: "Sunucu hatası".to_string(),
                error_code: "SERVER_ERROR".to_string(),
            }));
        }
    }

    let chunk_path = chunk_dir.join(format!("chunk_{}", info.chunk_number));
    let chunk_path_clone = chunk_path.clone();
    info!("Chunk kaydediliyor: {:?}", chunk_path);
    
    // Chunk'ı kaydet
    if let Err(e) = web::block(move || std::fs::write(&chunk_path_clone, &body)).await {
        error!("Chunk kaydedilemedi: {}", e);
        return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
            success: false,
            message: "Sunucu hatası".to_string(),
            error_code: "SERVER_ERROR".to_string(),
        }));
    }

    info!("Chunk başarıyla kaydedildi: {:?}", chunk_path);

    let all_chunks_uploaded = (1..=info.total_chunks)
        .all(|i| chunk_dir.join(format!("chunk_{}", i)).exists());

    if all_chunks_uploaded {
        info!("Tüm chunk'lar yüklendi, dosya birleştiriliyor");
        let file_name = format!("{}.{}", 
            info.file_id,
            Path::new(&info.file_name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("mp4")
        );
        
        let final_path = PathBuf::from(UPLOAD_DIR).join(&file_name);
        let final_path_clone = final_path.clone();
        info!("Final dosya yolu: {:?}", final_path);

        // İlk dosyayı oluştur
        if let Err(e) = web::block(move || std::fs::File::create(&final_path_clone)).await {
            error!("Final dosya oluşturulamadı: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                message: "Sunucu hatası".to_string(),
                error_code: "SERVER_ERROR".to_string(),
            }));
        }

        // Chunk'ları sırayla birleştir
        for i in 1..=info.total_chunks {
            let chunk_path = chunk_dir.join(format!("chunk_{}", i));
            let chunk_path_clone = chunk_path.clone();
            debug!("Chunk birleştiriliyor: {:?}", chunk_path_clone);
            
            let chunk_data = match web::block(move || fs::read(&chunk_path_clone)).await {
                Ok(Ok(data)) => data,
                Ok(Err(e)) => {
                    error!("Chunk okuma hatası: {}", e);
                    return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                        success: false,
                        message: "Sunucu hatası".to_string(),
                        error_code: "SERVER_ERROR".to_string(),
                    }));
                },
                Err(e) => {
                    error!("Chunk okunamadı: {}", e);
                    return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                        success: false,
                        message: "Sunucu hatası".to_string(),
                        error_code: "SERVER_ERROR".to_string(),
                    }));
                }
            };

            // Append modunda dosyaya yaz
            let final_path_clone = final_path.clone();
            if let Err(e) = web::block(move || {
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open(&final_path_clone)?;
                file.write_all(&chunk_data)
            }).await {
                error!("Chunk dosyaya yazılamadı: {}", e);
                return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                    success: false,
                    message: "Sunucu hatası".to_string(),
                    error_code: "SERVER_ERROR".to_string(),
                }));
            }
        }

        info!("Tüm chunk'lar başarıyla birleştirildi");
        let _ = web::block(move || fs::remove_dir_all(&chunk_dir)).await;

        let success_template = match fs::read_to_string("templates/success.html") {
            Ok(template) => template,
            Err(e) => {
                error!("Success template okunamadı: {}", e);
                return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                    success: false,
                    message: "Sunucu hatası".to_string(),
                    error_code: "SERVER_ERROR".to_string(),
                }));
            }
        };

        let file_url = file_name;
        let final_url = format!("/p/{}", file_url);
        let preview_html = format!(
            r#"<video width="100%" controls preload="metadata">
                <source src="/i/{}" type="video/mp4" />
                Tarayıcınız video etiketini desteklemiyor.
            </video>"#,
            file_url
        );
        
        let response_html = success_template
            .replace("{PREVIEW}", &preview_html)
            .replace("{FILE_URL}", &final_url);

        info!("Video yükleme işlemi başarıyla tamamlandı");
        return Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(response_html));
    }

    info!("Chunk {}/{} başarıyla yüklendi", info.chunk_number, info.total_chunks);
    Ok(HttpResponse::Ok().json(ChunkUploadResponse {
        success: true,
        message: format!("Chunk {}/{} başarıyla yüklendi", info.chunk_number, info.total_chunks),
        file_id: info.file_id.clone(),
    }))
}

async fn get_image(path: web::Path<String>) -> Result<NamedFile, Error> {
    let filename = path.into_inner();
    let file_path = PathBuf::from(UPLOAD_DIR).join(&filename);

    // Güvenlik kontrolü
    if !is_safe_path(&file_path) {
        return Err(actix_web::error::ErrorForbidden("Erişim reddedildi."));
    }

    match NamedFile::open(&file_path) {
        Ok(file) => Ok(file.set_content_type(mime_guess::from_path(&file_path).first_or_octet_stream())),
        Err(_) => Err(actix_web::error::ErrorNotFound("Dosya bulunamadı."))
    }
}

async fn get_preview(path: web::Path<String>) -> Result<HttpResponse, Error> {
    let filename = path.into_inner();
    let file_path = PathBuf::from(UPLOAD_DIR).join(&filename);

    // Güvenlik kontrolü
    if !is_safe_path(&file_path) {
        return Err(actix_web::error::ErrorForbidden("Erişim reddedildi."));
    }

    // Dosya uzantısını kontrol et
    if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
        match extension.to_lowercase().as_str() {
            "mp4" | "webm" | "mov" => {
                let preview_template = fs::read_to_string("templates/preview.html")?;
                
                // Video başlığını metadata.json dosyasından oku
                let file_stem = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let metadata_path = format!("{}/{}.json", UPLOAD_DIR, file_stem);
                let title = if let Ok(metadata) = fs::read_to_string(metadata_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&metadata) {
                        json["title"].as_str().unwrap_or("Video Önizleme").to_owned()
                    } else {
                        "Video Önizleme".to_owned()
                    }
                } else {
                    "Video Önizleme".to_owned()
                };

                let video_id = format!("{}.{}", file_stem, extension);
                let response_html = preview_template
                    .replace("{TITLE}", &title)
                    .replace("{BASE_URL}", SERVER_URL)
                    .replace("{VIDEO_ID}", &video_id);
                    
                Ok(HttpResponse::Ok()
                    .content_type("text/html; charset=utf-8")
                    .append_header(("Cache-Control", "no-cache, no-store, must-revalidate"))
                    .append_header(("Pragma", "no-cache"))
                    .append_header(("Expires", "0"))
                    .append_header(("X-Frame-Options", "ALLOWALL"))
                    .append_header(("Access-Control-Allow-Origin", "*"))
                    .body(response_html))
            },
            _ => Ok(HttpResponse::Found().append_header(("Location", format!("/i/{}", filename))).finish())
        }
    } else {
        Ok(HttpResponse::Found().append_header(("Location", format!("/i/{}", filename))).finish())
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info,info");
    env_logger::init();

    info!("Sunucu başlatılıyor...");
    info!("Server running at http://localhost:22994");

    HttpServer::new(|| {
        App::new()
            .app_data(web::JsonConfig::default().limit(MAX_FILE_SIZE))
            .app_data(web::PayloadConfig::new(MAX_FILE_SIZE))
            .app_data(web::FormConfig::default().limit(MAX_FILE_SIZE))
            .service(web::resource("/").to(index))
            .service(web::resource("/upload").to(upload))
            .service(web::resource("/upload/chunk").to(upload_chunk))
            .service(web::resource("/i/{filename}").to(get_image))
            .service(web::resource("/p/{filename}").to(get_preview))
    })
    .bind("127.0.0.1:22994")?
    .run()
    .await
}
