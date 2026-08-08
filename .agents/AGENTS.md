# NetLang Yapay Zeka Kuralları (Agent Rules)

Merhaba, bu projeye atanan yeni bir Yapay Zeka Ajanısın (veya eski oturumun devamısın).
Bu proje "NetLang" adında, hem SPICE tabanlı donanım simülasyonu yapabilen hem de otomatik şematik (Layout) çizebilen özel bir dil (DSL) projesidir.

## İlk Adımlar (Zorunlu)

1. **Önce Anayasayı Oku:** `docs/architecture.md` dosyasını okuyup proje bağlamını hafızana al.
2. **Sonra Yol Haritasını Oku:** `docs/ROADMAP.md` dosyasından mevcut geliştirme fazını ve yapılacak görevleri kontrol et.
3. **Faz atlama YASAKTIR.** Önceki fazın tüm görevleri tamamlanmadan sonraki faza geçilmez.

## Kırmızı Çizgiler (KESİNLİKLE Uyulacak Kurallar)

1. **Source, Battery Değil:** Voltaj kaynakları için ASLA `battery` kelimesi kullanılmaz, her zaman `source` kullan. Parser'da backward-compat olarak kabul edilir ama AST'de `Source`'a dönüşür.
2. **SPICE Düğüm Algoritması (Node Naming):** `graph.rs` içindeki düğüm isimlendirme algoritması (N_{İlkPin}) her zaman deterministik kalmalıdır. Buna müdahale etme veya bozma. User-named netler bu algoritmanın üzerine eklenir, algoritmayı değiştirmez.
3. **Orientasyon (Yönlendirme) Algoritması:** `layout.rs` dosyasındaki layout motoru yön farkındalığına sahiptir. Yeni bileşen eklerken `get_comp_def` içerisindeki pin (x,y) koordinatlarına ve `is_signal_pin` kurallarına uymak zorundasın.
4. **IR Tek Gerçek Kaynak:** Tüm backend'ler (SPICE, Layout, ERC, JSON) yalnızca Circuit IR (`ir.rs`) üzerinden çalışır. AST'den doğrudan backend çıktısı üretme.
5. **ERC, DRC Değil:** Schematic seviyesindeki kontroller **ERC** (Electrical Rules Check) olarak adlandırılır. `erc.rs` modülünü kullan.
6. **Test Kapısı:** Her değişiklikten sonra `cargo test` %100 geçmelidir. Başarısız test ile commit yapılmaz.
7. **Geriye Uyumluluk:** Yeni syntax eklerken mevcut `examples/*.nl` dosyaları kırılmamalıdır.

## Görev Takibi

Yaptığın her değişiklikten sonra `docs/ROADMAP.md`'deki ilgili checkbox'ı `[x]` olarak işaretle.

Bu kurallara uyarak NetLang'in mimarisini koruyabilirsin. Başarılar!
