# NetLang Yapay Zeka Kuralları (Agent Rules)

Merhaba, bu projeye atanan yeni bir Yapay Zeka Ajanısın (veya eski oturumun devamısın).
Bu proje "NetLang" adında, hem SPICE tabanlı donanım simülasyonu yapabilen hem de otomatik şematik (Layout) çizebilen özel bir dil (DSL) projesidir.

**Sistemi bozmamak için KESİNLİKLE uyman gereken kurallar:**

1. **Önce Anayasayı Oku:** Projeye başlarken veya mimari bir değişiklik yapmadan önce, MUTLAKA `docs/architecture.md` dosyasını okuyup proje bağlamını hafızana al.
2. **Battery değil Source:** Voltaj kaynakları için ASLA `battery` kelimesi kullanılmaz, her zaman `source` kullan.
3. **SPICE Düğüm Algoritması (Node Naming):** `graph.rs` içindeki düğüm isimlendirme algoritması (N_{İlkPin}) her zaman deterministik kalmalıdır. Buna müdahale etme veya bozma.
4. **Orientasyon (Yönlendirme) Algoritması:** `layout.rs` dosyasındaki DFS motoru yön farkındalığına sahiptir. Yeni bileşen eklerken `get_comp_def` içerisindeki pin (x,y) koordinatlarına ve `is_signal_pin` kurallarına uymak zorundasın.

Bu kurallara uyarak NetLang'in mükemmel mimarisini koruyabilirsin. Başarılar!
