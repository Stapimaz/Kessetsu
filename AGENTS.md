# Kessetsu Yapay Zeka Kuralları (Agent Rules)

Merhaba, bu projeye atanan yeni bir Yapay Zeka Ajanısın (veya eski oturumun devamısın).
Bu proje "Kessetsu" adında, hem SPICE tabanlı donanım simülasyonu yapabilen hem de otomatik şematik (Layout) çizebilen özel bir devre mühendisliği platformudur.

## İlk Adımlar (Zorunlu)

1. **Önce Anayasayı Oku:** `docs/architecture.md` dosyasını okuyup proje bağlamını hafızana al.
2. **Sonra Yol Haritasını Oku:** `docs/ROADMAP.md` dosyasından mevcut geliştirme fazını ve yapılacak görevleri kontrol et.
3. **Faz atlama YASAKTIR.** Önceki fazın tüm görevleri tamamlanmadan sonraki faza geçilmez.

## Kırmızı Çizgiler (KESİNLİKLE Uyulacak Kurallar)

1. **SPICE Düğüm Algoritması (Node Naming):** `graph.rs` içindeki düğüm isimlendirme algoritması deterministik kalmalıdır. User-named netler canonical kurallara göre otomatik adların önüne geçer; belirsizlikler diagnostic üretir.
2. **Orientasyon (Yönlendirme) Algoritması:** `layout.rs` dosyasındaki layout motoru yön farkındalığına sahiptir. Yeni bileşen eklerken `core/src/component.rs` içindeki ortak component kataloğunun pin koordinatlarına ve signal/through metadata'sına uy.
3. **IR Tek Gerçek Kaynak:** Tüm backend'ler (SPICE, Layout, ERC, JSON) yalnızca Circuit IR (`ir.rs`) üzerinden çalışır. AST'den doğrudan backend çıktısı üretme.
4. **ERC, DRC Değil:** Schematic seviyesindeki kontroller **ERC** (Electrical Rules Check) olarak adlandırılır. `erc.rs` modülünü kullan.
5. **Kalite Kapısı:** Değişiklikleri tamamlamadan önce kökten `powershell -ExecutionPolicy Bypass -File scripts/verify.ps1` çalıştır. İterasyon sırasında yalnız gerektiğinde `-SkipNpmInstall` kullan; final doğrulama canonical tam komutla yapılır.
6. **Geriye Uyumluluk:** Yeni syntax eklerken mevcut `examples/*.kess` dosyaları kırılmamalıdır.

## Görev Takibi

Yaptığın her değişiklikten sonra `docs/ROADMAP.md`'deki ilgili checkbox'ı `[x]` olarak işaretle.

Bu kurallara uyarak Kessetsu'nun mimarisini koruyabilirsin. Başarılar!
