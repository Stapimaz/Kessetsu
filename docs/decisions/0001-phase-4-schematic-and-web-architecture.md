# ADR 0001 — Faz 4 Şema ve Web Mimarisi

- Durum: Kabul edildi
- Tarih: 2026-08-13
- Kapsam: Web shell, SVG renderer, Schematic IR ve layout

## Bağlam

Faz 4 başlangıcındaki Web uygulaması gerçek WASM compile/ERC, deneysel SVG şema ve KiCad/SPICE indirme davranışlarına sahipti. Bununla birlikte bütün UI ve renderer `App.tsx` içinde, symbol geometrileri TypeScript'te tekrar tanımlı ve layout çıktısı `HashMap<component, position> + net_id/polyline` biçimindeydi. Bu veri pin endpoint, junction, bağlantısız crossing, net label, sürüm veya connectivity proof taşımıyordu.

Altı devrelik characterization corpus'u `layout_characterization` testiyle sabitlendi: minimal source/resistor, RC low-pass, Wheatstone bridge, op-amp gain stage, yüksek fan-out ve dört katlı 8 Ω power amplifier. Legacy layout'un bütün component ve bağlı netleri en azından bir polyline ile kapsadığı korunuyor; fakat yalnız bu, görsel geometrinin canonical graph ile eşdeğerliğini kanıtlamıyor.

Gerçek Chromium smoke testi iki görünmeyen entegrasyon hatasını ortaya çıkardı: Web'in `kessetsu.compile.v1` bekleyip Core'un `v2` üretmesi ve Monaco'nun CDN yüklemesinin ağsız/CSP ortamında hata vermesi. Compile sürümü artık WASM build'inden okunuyor; Monaco ve worker uygulamayla birlikte paketleniyor.

## Karar

| Katman | Karar | Gerekçe |
|---|---|---|
| Web shell | Refactor et | Çalışan WASM compile, editör ve temel panel davranışını koru; state/runtime/render/export sorumluluklarını ayrı modüllere böl. |
| React SVG renderer | Değiştir | Ayrı TypeScript symbol geometrisi drift kaynağıdır. Web yalnız Core'un versioned Schematic IR/SVG çıktısını sunacak ve etkileşim katmanı ekleyecek. |
| Legacy `LayoutResult` veri şekli | Değiştir | Typed endpoint, junction, crossing, label, ordering ve connectivity proof taşımadığı için public sözleşme olamaz. Yerine `kessetsu.schematic.v1` gelir. |
| Chain layout heuristic | Refactor et, corpus geçmezse değiştir | Basit rail/chain yerleşimi başlangıç heuristic'i olarak değerlidir. Deterministik layered placement, explicit net label ve orthogonal routing ile çevrelenecek; kalite raporu corpus kapısını geçmezse eski heuristic korunmayacak. |

Canonical sahiplik şudur: Circuit IR elektriksel gerçeğin, Schematic IR çizim/topoloji gerçeğinin sahibidir. Symbol/pin kataloğu Rust Core'dadır. Renderer ve exporter'lar Schematic IR tüketir; kendi pin listesi, connectivity veya layout algoritması kuramaz. Web state'i hiçbir zaman canonical artifact değildir.

## İlk dikey ve ana eval

- İlk dikey ürün yolu: `rc_filter.kess` → compile/ERC → şema → AC simulation → cutoff/assertions → export.
- Ana vizyon/eval yolu: `power_amplifier.kess` → 8 Ω yükte output power, gain, THD, clipping, device stress ve dissipation.

Bu iki devre birbirinin alternatifi değildir: RC entegrasyon sözleşmesini küçük yüzeyde kanıtlar, power amplifier ürünün gerçek mühendislik değerini sınar.

## Sonuçlar

- Legacy layout alanı yalnız migration süresince compatibility verisidir; yeni backend eklemek için kullanılmaz.
- Görsel kalite “güzel görünüyor” onayıyla kapanmaz. Connectivity, collision, crossing, bend ve determinism raporları otomatik kapıdır.
- Web'de çalışan ama CLI/Core'da karşılığı olmayan export biçimi eklenmez.

