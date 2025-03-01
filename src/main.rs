use actix_web::{web, App, HttpServer, HttpResponse, Error, HttpRequest};
use actix_multipart::Multipart;
use actix_files::NamedFile;
use futures::{StreamExt, TryStreamExt};
use rand::Rng;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use mime_guess;

const UPLOAD_DIR: &str = "uploads";
const URL_LENGTH: usize = 5;

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

    let field_opt = payload.try_next().await?;
    
    if let Some(mut field) = field_opt {
        let content_type = field.content_disposition().clone();
        
        if let Some(name) = content_type.get_name() {
            if name == "file" {
                if let Some(filename) = content_type.get_filename() {
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
                    
                    let filepath = format!("{}/{}.{}", UPLOAD_DIR, random_id, file_ext);
                    
                    let mut f = web::block(move || std::fs::File::create(&filepath))
                        .await
                        .unwrap()?;
                    
                    let mut file_size = 0;
                    while let Some(chunk) = field.next().await {
                        let data = chunk?;
                        file_size += data.len();
                        f = web::block(move || f.write_all(&data).map(|_| f)).await?.unwrap();
                    }
                    
                    if file_size > 0 {
                        let success_template = fs::read_to_string("templates/success.html")?;
                        let response_html = success_template.replace("{}.{}", &format!("{}.{}", random_id, file_ext));
                        
                        return Ok(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(response_html));
                    } else {
                        return Ok(HttpResponse::BadRequest().content_type("text/html; charset=utf-8").body("Dosya boş görünüyor. Lütfen geçerli bir medya dosyası seçin."));
                    }
                } else {
                    return Ok(HttpResponse::BadRequest().content_type("text/html; charset=utf-8").body("Dosya adı bulunamadı."));
                }
            }
        }
    }
    
    Ok(HttpResponse::BadRequest().content_type("text/html; charset=utf-8").body("Lütfen bir medya dosyası seçin ve tekrar deneyin."))
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    println!("Server running at http://localhost:22994");

    HttpServer::new(|| {
        App::new()
            .service(web::resource("/").to(index))
            .service(web::resource("/upload").to(upload))
            .service(web::resource("/i/{filename}").to(get_image))
    })
    .bind("127.0.0.1:22994")?
    .run()
    .await
}
