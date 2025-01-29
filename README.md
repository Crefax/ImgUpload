# Rust Resim Yükleme Servisi

Bu proje, [Cursor AI](https://cursor.sh/) yardımıyla geliştirilmiş modern bir resim yükleme web servisidir. Rust programlama dili ve Actix-web framework'ü kullanılarak oluşturulmuştur.

## Özellikler

- 🌓 Koyu/Açık tema desteği (localStorage ile tema tercihi kaydedilir)
- 📤 Çoklu yükleme yöntemleri:
  - Sürükle-bırak
  - Dosya seçici
  - Panodan yapıştırma (Ctrl+V)
- 🖼️ Resim önizleme
- 🔗 Otomatik URL oluşturma (5 karakterli benzersiz kod)
- 📋 Tek tıkla URL kopyalama
- 🎨 Modern ve duyarlı arayüz tasarımı

## Teknolojiler

- 🦀 Rust
- 🌐 Actix-web Framework
- 📁 Multipart dosya yükleme
- 🎯 JavaScript (dosya işleme, tema yönetimi)
- 📱 Responsive tasarım

## Kurulum

1. Rust'ı yükleyin (https://rustup.rs/)
2. Projeyi klonlayın:
   ```bash
   git clone [repo-url]
   cd rust-upload
   ```
3. Projeyi derleyin ve çalıştırın:
   ```bash
   cargo run
   ```
4. Tarayıcınızda http://localhost:8080 adresine gidin

## Kullanım

1. Ana sayfada üç farklı yöntemle resim yükleyebilirsiniz:
   - Resim dosyasını sürükleyip bırakın
   - "Resim seçmek için tıklayın" alanına tıklayın
   - Herhangi bir resmi kopyalayıp Ctrl+V ile yapıştırın

2. Yüklenen her resim için benzersiz bir URL oluşturulur
3. URL'yi kopyala butonu ile bağlantıyı kolayca paylaşabilirsiniz

## Klasör Yapısı

```
rust-upload/
├── src/
│   └── main.rs           # Ana uygulama kodu
├── templates/
│   ├── index.html        # Ana sayfa şablonu
│   └── success.html      # Başarılı yükleme sayfası şablonu
├── uploads/              # Yüklenen dosyaların saklandığı klasör
├── Cargo.toml           # Bağımlılık ve proje yapılandırması
└── README.md            # Proje dokümantasyonu
```

## Geliştirici Notu

Bu proje, Cursor AI'ın yardımıyla geliştirilmiş olup, modern web geliştirme pratiklerini ve Rust'ın güçlü özelliklerini sergilemektedir. Cursor AI, kod önerileri ve en iyi uygulamalar konusunda rehberlik sağlamıştır.

## Lisans

Bu proje MIT lisansı altında lisanslanmıştır. Daha fazla bilgi için `LICENSE` dosyasına bakın. 