# Export Sözleşmesi ve Format Matrisi

Kessetsu'nun export katmanı Web'e veya CLI'a ait değildir. Bütün çıktılar typed Circuit IR'den üretilen ve bağlantısı doğrulanan `kessetsu.schematic.v1` üzerinden `kessetsu.export.v1` sözleşmesiyle hazırlanır. CLI ve Web yalnız bu ortak Core API'sini çağırır.

Her artifact şu kanıtları taşır: exporter adı/sürümü, MIME ve uzantı, byte uzunluğu, SHA-256, `connectivity_verified`, capability alanları, warning listesi ve bilinen semantik kayıplar. Desteklenmeyen topoloji veya sembol sessizce yaklaşık çizilmez; `KES-Xxxx` diagnostic ile export durur.

## İlk yayın formatları

| Format | Amaç | Connectivity | Model | Analysis | Doğrulama / açık kayıp |
|---|---|---:|---:|---:|---|
| SVG | Ölçeklenebilir görsel, doküman ve Web | Evet | Hayır | Hayır | Semantic text, sabit `viewBox`, açık renk stili |
| PNG | Sunum, rapor, hızlı paylaşım | Görsel izdüşüm | Hayır | Hayır | Pure-Rust canonical SVG raster; `0.25..8` scale, beyaz/transparan arka plan |
| PDF | Baskı ve vektör doküman | Görsel izdüşüm | Hayır | Hayır | Tek sayfa, içerik sınırı, auto orientation, Schematic IR margin'i, deterministic vector glyphs; multi-page yok |
| Schematic IR JSON | Kayıpsız makine alışverişi | Evet | Evet | Hayır | `kessetsu.schematic.v1`, deterministic pretty JSON |
| SPICE | Simülasyon ve otomasyon | Evet | Evet | Evet | Canonical Ngspice netlist |
| KiCad `.kicad_sch` | Düzenlemeye devam etme | Evet | Metadata | Hayır | KiCad 10 parser/netlist/ERC smoke; portable embedded symbols symbol-table warning üretebilir |
| LTspice `.asc` | Düzenleme ve LTspice simülasyonu | Evet | Evet | Evet | LTspice 24.1.9 gerçek `-netlist` smoke; assertions `.kess` kaynağında kalır |

PDF politikası bilerek tek sayfadır: elektronik şema bölünerek basılırsa bağlantı takibi kötüleşir. Çok büyük şemada Core önce okunabilir Schematic IR üretmek zorundadır; exporter keyfî sayfa kırılımı icat etmez. İlk sürüm A4/Letter çerçevesi yerine canonical içerik boyutunda media box kullanır; böylece boş alan taşınmaz ve orientation içerikten doğal olarak çıkar.

Raster ve vektör font ölçümü sistem fontuna bırakılmaz. Repo içindeki SIL OFL 1.1 lisanslı Roboto Mono, native ve WASM render sırasında aynı biçimde yüklenir. SVG'deki semantic text tarayıcıda seçilebilir kalır; PNG piksel çıktısıdır, PDF ise platforma bağlı font-subset kimliği üretmemesi için glyph'leri deterministic vektör path'e çevirir.

## EDA doğrulama

`scripts/verify-eda-exports.ps1 -RequireApplications`, RC filter, gain-stage ve power-amplifier fixture'larını Core/CLI'dan üretir. Sonra:

- KiCad 10 ile dosyayı parse eder, KiCad XML netlist üretir, ERC çalıştırır ve bütün component reference'larının kaldığını doğrular.
- LTspice 24.1.9 ile `.asc` dosyasını `-netlist` yolundan gerçekten açar; oluşan `.net` içinde bütün component reference'larını ve tamamlanmış `.end` kaydını doğrular.

Uygulamalar bulunmuyorsa normal yerel doğrulama bunu açıkça `skipped` raporlar; release makinesinde `-RequireApplications` zorunludur. Core unit testleri ayrıca bütün formatların byte-determinism, signature, schema, hash ve connectivity sözleşmesini platformdan bağımsız kontrol eder.

KiCad modern s-expression şema formatını ve `.kicad_sch` uzantısını resmi olarak belgeler: <https://dev-docs.kicad.org/en/file-formats/sexpr-schematic/>. LTspice `.asc` şema ve `.net/.cir/.sp` netlist dosyalarını uygulama formatları olarak tanımlar: <https://ltspicehelpmanual.azurewebsites.net/introduction1.htm>.

## Değerlendirilen fakat ilk yayına alınmayan hedefler

| Hedef | Karar | Gerekçe |
|---|---|---|
| Qucs-S `.sch` | Faz 5+ adapter adayı | Qucs-S şemayı proje girdisi olarak belgeliyor ancak bu sürümde kurulu hedef uygulamayla round-trip kanıtı yok: <https://qucs-s-help.readthedocs.io/en/latest/overview/understanding-file-structure.html> |
| CircuitJS | Faz 5+ adapter adayı | Açık metin/URL devre aktarımı var; bileşen ve analog model semantiği Kessetsu kapsamıyla bire bir değil. Resmi kaynak: <https://github.com/sharpie7/circuitjs1> |
| EasyEDA JSON | Faz 5+ adapter adayı | Format açık ve belgeli olsa da sıkıştırılmış primitive sözleşmesi/editör sürümü bakım maliyeti yüksek; gerçek import smoke olmadan destek ilan edilmiyor: <https://docs.easyeda.com/en/DocumentFormat/EasyEDA-Document-Format/> |
| EDIF | İlk yayın dışında | Genel değişim standardı olsa da hedef uygulama ve analog schematic davranışı üzerinde doğrulanmış, düşük-kayıplı bir akış yok |

Bu kararlar format sayısını küçük tutmak için değil, “download düğmesi var” ile “mühendislik verisi taşındı” arasındaki farkı korumak içindir. Yeni adapter ancak canonical graph bağlantı fixture'ı ve hedef uygulama smoke testiyle desteklenen formata yükselir.

## CLI

```powershell
kess render circuit.kess --output circuit.svg
kess render circuit.kess --output circuit.png --scale 3 --background transparent
kess render circuit.kess --output circuit.pdf

kess export circuit.kess --target schematic-json --output circuit.schematic.json
kess export circuit.kess --target spice --output circuit.spice
kess export circuit.kess --target kicad --output circuit.kicad_sch
kess export circuit.kess --target ltspice --output circuit.asc
```

Var olan hedefi değiştirmek için açıkça `--force` gerekir. Kaynak dosyanın üstüne yazma her durumda reddedilir. `--format json` kullanıldığında çıktı dosyasının kendisi stdout'a karıştırılmaz; agent yalnız structured artifact metadata ve diagnostics görür.
