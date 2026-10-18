<div align="center">

# ECOnnector

**Mac, Windows ve Android cihazlarını tek bir ekosistemde birleştir.**
Her şeyi anında gönder, dosyaları kopya oluşturmadan senkron tut, panonu paylaş
ve yaptığın işe diğer cihazda kaldığın yerden devam et.

[![Core](https://github.com/SerhanEnsar/ecosystem-connector/actions/workflows/core.yml/badge.svg)](https://github.com/SerhanEnsar/ecosystem-connector/actions/workflows/core.yml)
[![Android](https://github.com/SerhanEnsar/ecosystem-connector/actions/workflows/android.yml/badge.svg)](https://github.com/SerhanEnsar/ecosystem-connector/actions/workflows/android.yml)
[![Lisans: Apache-2.0](https://img.shields.io/badge/lisans-Apache--2.0-blue.svg)](LICENSE)

[English](README.md) · [Mimari](docs/ARCHITECTURE.md) · [Katkı](CONTRIBUTING.md) · [Güvenlik](SECURITY.md)

</div>

> [!NOTE]
> Proje **erken geliştirme** aşamasında, henüz kullanılabilir değil. Demolar, ekran
> görüntüleri ve hız grafikleri özellikler geldikçe eklenecek. Gösterilen her rakam
> [`bench/`](bench/) içindeki tekrarlanabilir ölçümlerden gelecek.

## Neden

Bilgisayar, telefon ve tablet arasında çalışmak, aynı dosyayı ayrı araçlarla tekrar tekrar
göndermek demek. Bilgisayarda bir PDF'i düzenliyorsun, tablete gönderiyorsun, yine
düzenliyorsun, yine gönderiyorsun. Sonunda her cihazda farklı bir kopya birikiyor.

Platform ekosistemleri bunu yalnızca kendi cihazları arasında çözüyor. ECOnnector
bunu platformlar arasında yapar: Mac ↔ Android, Android ↔ Android, Windows ↔ Mac,
Windows ↔ Android.

## Özellikler (planlanan)

| Özellik | Ne yapar |
|---|---|
| **Hızlı gönderim** | Her türlü dosya, klasör, link ya da metni yakındaki cihaza gönderir. Bağlantı koparsa aktarım kaldığı yerden devam eder. |
| **Kablo hızlandırma** | USB-C kablo takıldığında (USB tethering ya da Thunderbolt köprüsü) süren aktarım yeniden başlamadan daha hızlı yola geçer. |
| **Bağlı dosyalar** | Her cihazda tek dosya, tek sürüm. Herhangi birinde düzenle, diğerleri saniyeler içinde güncellensin. Kopya birikmez. |
| **Ortak pano** | Bir cihazda kopyala, diğerinde yapıştır. Açık tarayıcı sekmesini gönder. |
| **Handoff** | Diğer cihazda açık olan dosya ya da web sayfası için "Bu cihazda devam et?" sorusu. |

## Kurulum

En son sürümü [Releases](https://github.com/SerhanEnsar/ecosystem-connector/releases)
sayfasından indir:

| Platform | Dosya |
|---|---|
| Android telefon veya tablet | `…-android-arm64-v8a.apk` (çoğu cihaz) ya da `…-android-universal.apk` |
| macOS | `…-macos-universal.zip` |
| Windows | `…-windows-x64.zip` |
| Komut satırı aracı (isteğe bağlı) | `…-command-line-…` |

İlk sürümler ön sürüm (pre-release) olarak işaretlenir. Uygulamalar eşleşebilir (QR kod
ya da link), eşleşmiş cihazları görür, dosya gönderip alabilir; `ec` aracı da çalışır.

## Nasıl çalışır

- **Önce yerel ağ.** Cihazlar birbirini ağında bulur. Bulut sunucusu yok, veriler
  cihazlarından çıkmaz.
- **Uçtan uca şifreli.** Cihazlar bir kez QR kod ve doğrulama koduyla eşleşir. Sonraki her
  bağlantı sabitlenmiş anahtarlarla karşılıklı TLS kullanır. Eşleşmemiş cihazlar reddedilir.
- **Hızlı.** Dosyalar parçalara bölünür ve paralel QUIC akışlarıyla gönderilir. Her parça
  BLAKE3 ile doğrulanır. Kesilen bir aktarımda yalnızca eksik parçalar, düzenlenen bir
  dosyada yalnızca değişen parçalar gönderilir.

Ayrıntılar [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) içinde (İngilizce).

## Lisans

[Apache Lisansı, Sürüm 2.0](LICENSE) ile lisanslanmıştır. Telif hakkı 2026 SerhanEnsar.
