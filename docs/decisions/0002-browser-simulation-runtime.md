# ADR 0002 — Browser Simulation Runtime

- Durum: Kabul edildi
- Tarih: 2026-08-13
- İlk yayın kararı: Ngspice WASM, Web Worker içinde

## Değerlendirilen yollar

| Ölçüt | `eecircuit-engine` 1.7.0 | Kendi Ngspice/Emscripten build'i | Service-backed native Ngspice |
|---|---|---|---|
| Motor | Ngspice WASM wrapper | Ngspice WASM | Native Ngspice |
| Yerel artifact | npm paketi 40,693,211 byte unpacked; ana ESM bundle yaklaşık 20.4 MB | Build seçeneklerine bağlı | Browser artifact'i küçük; server/container gerekir |
| Startup | İlk worker/WASM parse maliyeti; ölçüm 4.2 parity testinde kapı | Benzer, ayrıca build zinciri sahipliği | Network + cold start |
| Cancellation | Worker terminate/restart ile sert sınır | Worker terminate/restart ile sert sınır | HTTP abort tek başına server işini durdurmaz; server cancellation gerekir |
| Model desteği | Ngspice netlist/model semantiği; NetLang security/parity corpus'uyla doğrulanacak | Build flags'e bağlı | Native ile en geniş ve mevcut parity |
| Offline/privacy | Evet; circuit browser dışına çıkmaz | Evet | Hayır; source/netlist server'a gider |
| Deployment | Static asset + worker MIME/cache | Static asset + worker MIME/cache; ayrıca reproducible Emscripten build | Stateful/isolated execution service, queue ve abuse kontrolü |
| Supply-chain | Exact npm version/integrity + upstream source/license | Kaynak commit, toolchain ve build recipe tamamıyla bize ait | OS/container Ngspice provenance |

## Karar ve gerekçe

İlk public Web Hub, `eecircuit-engine` **1.7.0 exact** paketini yalnız Web Worker içinde çalıştıracak. Paket aktif EEcircuit projesinde gerçek browser simulation için kullanılıyor ve wrapper MIT lisanslı. Ngspice kodunun büyük kısmı modified BSD lisanslıdır; dağıtılan artifact için transitive license notice ve exact hash ayrıca tutulacaktır.

Bu seçim kalıcı domain bağımlılığı değildir. Worker adaptörü yalnız canonical SPICE ve typed request alır, ham engine sonucunu `netlang.simulation.v1` şekline çevirir. Measurement/assertion WASM Core içinde aynı typed dataset üzerinde çalışır. UI `eecircuit-engine` tiplerini görmez. İleride self-built runtime veya service adapter'a geçiş bu sınırın arkasında kalır.

Service-backed yol ilk yayın için reddedildi: zero-friction hedefini karşılasa da circuit verisini ağ üzerinden taşır, ayrı güvenli process servisi ve operasyon yüzeyi yaratır. Native parity sorunu yaşanırsa sessiz fallback yapılmayacak; Web structured unsupported/runtime diagnostic gösterecek.

Wokwi'nin `ngspice-wasm` build recipe'si teknik feasibility kanıtıdır fakat repo tek commitli, release artifact/API sözleşmesi sunmuyor ve son source push'u 2022'de. İlk yayın runtime bağımlılığı olarak seçilmedi. Uzun vadede supply-chain kontrolü gerekirse aynı adapter arkasında reproducible self-build adayıdır.

## Zorunlu kabul kapıları

1. Worker ana thread'i bloke etmez; timeout/cancel worker'ı terminate edip temiz instance başlatır.
2. Runtime init çıktısından simulator provenance görünür olur; package version ve asset SHA-256 kaydedilir.
3. RC, gain-stage ve power-amplifier canonical netlist'leri native/Web aynı PASS/FAIL kararını tanımlı toleransta verir.
4. Raw log debug opt-in'dir; büyük dataset iki kez UI state'e kopyalanmaz.
5. Runtime/model license notice public artifact ile dağıtılır.

## Artifact güncelleme ve cache politikası

- Runtime bağımlılığı floating range değil exact `eecircuit-engine@1.7.0` olarak kilitlidir. Güncelleme yalnız package integrity, ESM SHA-256, browser parity ve lisans inventory'si birlikte gözden geçirilerek yapılır.
- `runtime-manifest.json` package integrity yanında dağıtılan 20,424,332 byte ESM kaynağının SHA-256 değerini taşır. Hash değişimi sürüm değişmeden gerçekleşirse supply-chain drift kabul edilir ve build durdurulur.
- Production'da content-hash taşıyan JS/WASM/Worker asset'leri `public,max-age=31536000,immutable`; HTML, runtime manifest ve notice dosyaları `no-cache` ile sunulur. Yeni runtime eski hashed asset'i yerinde değiştirmez.
- Vite release build'i EEcircuit MIT metni ile tam Ngspice licensing inventory'sini `dist/licenses/` altına kopyalar.

## Kaynaklar

- [Ngspice FAQ ve shared-library/lisans bilgisi](https://ngspice.sourceforge.io/faq.html)
- [Ngspice geliştirici ve lisans bilgisi](https://ngspice.sourceforge.io/devel.html)
- [EEcircuit browser uygulaması](https://github.com/eelab-dev/EEcircuit)
- [`eecircuit-engine` kaynak reposu](https://github.com/eelab-dev/EEcircuit-engine)
- [Wokwi Ngspice WASM build recipe](https://github.com/wokwi/ngspice-wasm)
