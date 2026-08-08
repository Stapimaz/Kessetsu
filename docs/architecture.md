# NetLang Projesi Mimari Anayasası

Bu doküman, NetLang projesinin çekirdek algoritmalarını, derleme süreçlerini ve dil tasarım standartlarını açıklar. **Projeye dahil olan tüm geliştiriciler ve Yapay Zeka Ajanları (AI Agents), projede herhangi bir kod yazmadan önce bu dokümandaki kurallara uymak ZORUNDADIR.**

> **Geliştirme planı ve görev takibi için:** `docs/ROADMAP.md` dosyasına bakınız.

## 1. Sistemin Temel Parçaları

NetLang projesi temelde iki ana parçadan oluşur:
1. **`netlang-core` (Rust):** Dilin ayrıştırıcısı (parser), AST oluşturucusu, Circuit IR dönüştürücüsü, ERC (Electrical Rules Check) motoru, SPICE netlist jeneratörü ve otomatik şema (layout) motorunu barındıran çekirdek kütüphane.
   - Hem bir kütüphane (`lib`), hem CLI (`bin/`) olarak hem de Web için WASM (`wasm.rs`) olarak derlenir.
2. **`webapp` (React/TS):** Kullanıcının kodu yazdığı ve sonuçları (şema/grafik) gördüğü UI. `netlang-core`'u WASM üzerinden tarayıcı içinde gerçek zamanlı çalıştırır.

### Derleme Pipeline'ı

```
NetLang Source (.nl)
    │
    ▼
  Parser (netlang.pest → pest → AST)
    │
    ▼
  AST → Circuit IR (typed dönüşüm)
    │
    ├──► ERC (structural kontroller)
    ├──► SPICE Netlist üretimi
    ├──► Layout (şema yerleşim)
    └──► JSON Output (API/Agent)
```

**Kritik kural:** Tüm backend'ler (SPICE, Layout, ERC, JSON) yalnızca Circuit IR üzerinden çalışır. AST'den doğrudan çıktı üretilmez.

## 2. Dilin Sözdizimi (Syntax) ve Kurallar

### Desteklenen Bileşenler
| Keyword | Tür | Pinler | SPICE Prefix |
|---|---|---|---|
| `resistor` | Pasif | p1, p2 | R_ |
| `capacitor` | Pasif | p1, p2 | C_ |
| `inductor` | Pasif | p1, p2 | L_ |
| `diode` | Yarı-iletken | p1, p2 | D_ |
| `transistor` | BJT (NPN/PNP) | c, b, e | Q_ |
| `mosfet` | MOSFET (NMOS/PMOS) | d, g, s | M_ |
| `opamp` | Op-Amp | in_p, in_n, out, vcc, vee | X_ |
| `source` | Voltaj Kaynağı | plus, minus | V_ |
| `current_source` | Akım Kaynağı | plus, minus | I_ |

### Temel Kurallar
- **ÖNEMLİ KURAL:** Piller veya AC/DC tüm voltaj kaynakları için `battery` KELİMESİ KULLANILMAZ. Yerine **`source`** kullanılır. SPICE'ta hepsi `V_` ile ifade edilir. (Parser geriye uyumluluk için `battery` kabul eder ama AST'de `Source`'a dönüştürür.)
- **Değerler:** Bileşen değerleri typed olarak parse edilir. SI prefixler desteklenir:
  - `resistor R1 10k` → 10000 Ω
  - `capacitor C1 100uF` → 100µF
  - `source Vin 5V` → 5V DC
  - `source Vin sine(0V, 1V, 1kHz)` → AC sinüs kaynağı
  - Geriye uyumluluk: `source Vin "SINE(0 1V 1kHz)"` da kabul edilir (string olarak)
- **Simülasyon Komutları:** `simulate <cmd> <args*>`
  - `simulate op` — DC Operating Point
  - `simulate tran 10us 1ms` — Transient
  - `simulate ac dec 10 1Hz 1MHz` — AC Analiz
- **Assertion'lar (Test):**
  - `assert max(V(out)) < 3.3V`
  - `assert peak(I(D1)) < 100mA`

## 3. Circuit IR (Intermediate Representation)

AST ile SPICE/Layout/ERC arasında **typed bir ara katman** bulunur. Bu katmanın amacı:

1. **Tip güvenliği:** `value: String` yerine typed parameters (resistance, capacitance, waveform, vb.)
2. **Statik doğrulama:** IR üzerinde SPICE çalıştırmadan kontrol yapılabilir
3. **Backend bağımsızlığı:** Syntax değişse bile backend'ler etkilenmez
4. **Agent erişimi:** AI ajanları doğrudan IR JSON'ı üretebilir/okuyabilir

**Kural:** Hiçbir backend, IR'yi atlayarak doğrudan AST üzerinden çalışmamalıdır.

## 4. Düğüm (Node) İsimlendirme Algoritması (`graph.rs`)

Component pin adları, canonical backend sırası, SPICE prefix'i ve layout pin koordinatları `core/src/component.rs` kataloğunda tek kez tanımlanır. Graph, ERC, SPICE ve layout kendi ayrı pin listelerini üretmez. Fiziksel bağlantısı olmayan bir pin magic integer ile değil `Option<NetId>::None` ile temsil edilir; `NetId(0)` typed ground kimliğidir.

SPICE motoru için düğüm isimleri rastgele tam sayılar DEĞİLDİR. Okunabilirlik ve determinism için özel bir algoritma kullanılır:

1. Explicit `net GND` hattına bağlı her şey her zaman `"0"` düğümündedir ve bu referans legacy fallback'ten önceliklidir. Explicit GND yoksa lexicographic olarak ilk voltage-source `minus` neti geriye uyumluluk fallback'i olur. Birden fazla bağımsız aday deterministic seçilse bile `NL-E008` ambiguity diagnostic üretilir; belirsizlik sessizce başarılı sayılmaz.
2. **User-named netler birinci sınıf kimliktir.** Kullanıcı `net output` tanımladıysa, o net SPICE'ta `output` olarak görünür.
   - Component ve net aynı exact identifier'ı paylaşamaz (`NL-E006`).
   - Aynı fiziksel nete birden fazla user-name bağlanamaz (`NL-E007`).
3. User ismi olmayan netlerde, kendisine bağlı pinlerin listesi **alfabetik olarak sıralanır** ve en baştaki pinin adı alınır.
4. Pin adındaki nokta `.` karakteri alt çizgiye `_` çevrilip başına `N_` eklenir.
   - *Örnek:* Bir düğüme `R1.p2`, `C1.p1` ve `Q1.b` bağlıysa. Alfabetik sırada ilk gelen `C1.p1`'dir. Düğüm ismi **`N_C1_p1`** olur. SPICE çıktısında voltaj `v(N_C1_p1)` olarak okunur.

**Determinism kuralı:** Aynı canonical graph her zaman aynı net isimlerini üretir. Bu algoritma BOZULMAMALIDIR. Ancak user-named netler bu algoritmanın üstüne eklenir — otomatik isimler yalnızca isimsiz netler için fallback'tir.

**Not:** Algoritma deterministiktir ama edit-stable değildir. Devreye yeni component eklendiğinde otomatik net isimleri değişebilir. Önemli ölçüm noktaları için user-named net kullanılmalıdır.

## 5. SPICE Motoru ve Standart Modeller

Ngspice entegrasyonu gömülü çalışır.
- Eğer IR içerisinde `2N3904`, `1N4148`, `IRF540` gibi bilinen bir parça kullanılırsa, SPICE jeneratörü bu parçanın `.model` veya `.subckt` tanımını otomatik olarak netlist'in sonuna ekler. Kullanıcıların `.model` yazmasına gerek yoktur.
- **Model provenance:** Her modelin kaynağı (builtin/user-defined) ve tipi (NPN/PNP/NMOS/PMOS/D) IR'de belirtilir.
- **Model kalitesi:** Dahili modeller "generic" kalitededir. İleri sürümlerde üretici-spesifik modeller ve kalite seviyeleri eklenecektir.
- **Canonical sayılar:** Generated SPICE sayıları tek formatter kullanır. Orta büyüklükler trimlenmiş decimal, çok küçük/büyük değerler normalize edilmiş lowercase exponent ile yazılır; `-0` ve binary float artıkları output'a taşınmaz.

### Desteklenen Modeller
| Model | Tür | Kaynak |
|---|---|---|
| 2N3904 | BJT NPN | Builtin |
| 2N3906 | BJT PNP | Builtin |
| 2N2222 | BJT NPN | Builtin |
| 1N4148 | Diode | Builtin |
| 1N4007 | Diode | Builtin |
| IRF540 | MOSFET NMOS | Builtin |

User-defined model declaration/include syntax'ı henüz tanımlı değildir. Bu nedenle bilinmeyen model adları IR'ye raw string olarak geçirilmez; `NL-C003` semantic diagnostic ile reddedilir. BJT/MOSFET/diode için yukarıdaki builtin modeller ve güvenli default'lar kullanılır. Op-amp syntax'ı parser ve component modelinde tanımlıdır ancak doğrulanmış bir builtin subcircuit henüz bulunmadığı için modelsiz op-amp `NL-C005` ile fail-closed davranır; boş `X_` SPICE satırı üretilmez.

## 6. ERC (Electrical Rules Check) Motoru (`erc.rs`)

> **Not:** Daha önceki sürümlerde "DRC" olarak adlandırılıyordu. Schematic seviyesindeki kontroller için doğru terim **ERC** (Electrical Rules Check). DRC, PCB physical design kontrolleri için kullanılır.

### Yapısal Kontroller (Simülasyonsuz)
| Kod | Severity | Açıklama |
|---|---|---|
| NL-E001 | Error | Duplicate component declaration |
| NL-E002 | Error | Undefined component reference |
| NL-E003 | Error | Floating pin (bağlantısız zorunlu pin) |
| NL-E004 | Error | Direct short circuit (source plus=minus) |

### Simülasyon Tabanlı Kontroller (İleri sürüm)
| Kod | Severity | Açıklama |
|---|---|---|
| NL-S001 | Warning | Voltaj limit aşımı |
| NL-S002 | Warning | Akım limit aşımı |
| NL-S003 | Error | DC operating point bulunamadı |
| NL-S004 | Warning | Convergence problemi |

### Çıktı Formatı
- **İnsan modu (varsayılan):** Renkli, span bilgili, suggestion içeren mesajlar
- **JSON modu (`--format json`):** Makine-okunabilir structured diagnostics
  ```json
  {
    "code": "NL-E003",
    "severity": "error",
    "message": "Floating Pin: R1.p1 is not connected to anything.",
    "component": "R1",
    "pin": "p1"
  }
  ```

**Önemli not:** ERC sonuçları, tespit edilen riskleri raporlar. Bu sonuçlar fiziksel doğrulama veya mühendis incelemesinin yerine geçmez. Özellikle thermal davranış, PCB parasitikleri, ESD ve üretici toleransları gibi konular ERC kapsamı dışındadır.

## 7. Şema (Layout) Motoru (`layout.rs`)

Şematiği çizerken parçaları x/y koordinatlarına yerleştirmek için **chain-based vertical layout** yaklaşımı kullanılır. DFS, layout pipeline'ında traversal ve başlangıç sıralaması için kullanılan heuristic'lerden biridir.

- **KURAL:** Algoritma "Yön Farkındalığına (Orientation Awareness)" sahiptir.
  - GND pinlerine doğru olan yollar her zaman AŞAĞI yönlendirilir.
  - Çıkış (Output) pinleri SAĞA, Giriş (Input) pinleri SOLA bakmalıdır.
  - Sinyal akışında `is_signal_pin` fonksiyonu kullanılarak yollar önceliklendirilir. (Örneğin BJT'de `b` pini sinyal girişi sayılır, Akım `c`'den `e`'ye akar.)
- **Connectivity doğrulaması:** Layout çıktısı, orijinal netlist ile bağlantısal eşdeğerlik açısından doğrulanmalıdır (round-trip check).
- **Bilinen sınırlamalar:** Döngüsel topolojiler (Wheatstone bridge, feedback loop), çok yüksek fan-out ve bidirectional sinyaller için layout kalitesi düşebilir. Bu topolojiler için gelişmiş algoritmalar (Sugiyama/layered) ileride eklenecektir.

## 8. NetLang Vizyonu ve Ekosistem Manifestosu

**NetLang**, analog ve karma-sinyal devrelerini yazılım gibi derlemek, simüle etmek ve assertion'larla test etmek için tasarlanmış; native ve browser ortamlarında çalışan, AI-agent odaklı, deterministik bir SPICE derleyicisi ve doğrulama altyapısıdır.

```
Compile, simulate and test circuits like software.
```

Bu ekosistem üç sütun üzerinde yükselir:

### 1. NetLang Core (Rust Çekirdeği)
Projenin kalbi. Parser, IR, ERC, SPICE jeneratör ve layout motoru tek bir Rust crate içinde yaşar. Hem kütüphane (`lib`), hem CLI, hem WASM olarak derlenir. Deterministik davranış sağlar — aynı devre, her platformda aynı sonucu üretir.

### 2. NetLang CLI (Yapay Zeka ve Geliştiriciler İçin Motor)
Hiçbir dış kurulum veya internet bağlantısı gerektirmeyen, içine endüstri standardı simülasyon fizik motoru (Ngspice) gömülmüş tek parça bir Rust derleyicisidir.

- **Dağıtım:** İndirilebilir tek binary (`netlang`), `cargo install`, ileride VS Code extension.
- **Kullanım:** AI ajanları ve donanım mühendisleri devreyi derlemek, test etmek ve otomatik JSON formatında hataları ayıklamak için kullanır. CLI komutları, exit kodları ve JSON yapısı hakkında detaylı bilgi için lütfen [CLI Reference (docs/cli_reference.md)](file:///c:/Users/stapi/OneDrive/Belgeler/NetLang/docs/cli_reference.md) dosyasına bakınız.
- **TDD Döngüsü:** Ajan, assertion'ları yazılım testleri gibi kullanarak devreyi iteratif olarak düzeltebilir (Self-Healing). Her iterasyonda structured feedback alır.

### 3. NetLang Web Hub (İnsanlar İçin Vitrin ve Oyun Alanı)
Kullanıcıların kayıtsız, indirmesiz kullanabildiği; Rust çekirdeğini WASM ile tarayıcıda çalıştıran arayüz.

- **Playground:** Hızlı prototip testi. Kod yaz → anında ERC + şema + simülasyon.
- **Paylaşım:** Devre kodu URL'ye gömülür. Linke tıklayan herkes devreyi kendi tarayıcısında çalıştırabilir.
- **API Vitrini:** Motorun hızını ve doğruluğunu gösteren canlı demo.

**Güvenlik notu:** Web playground'da kullanıcı girdisi doğrudan SPICE string olarak netlist'e eklenmez. Tüm girdiler IR üzerinden typed olarak işlenir. Raw SPICE erişimi (ileride `unsafe spice_raw {}`) web sürümünde varsayılan olarak kapalıdır.

**ÖZETLE:** NetLang bir "çizim programı" değil, bir devre derleyicisi ve doğrulama altyapısıdır. Yapay zeka ajanları için terminalde otonom çalışan bir simülasyon/test motoru; insanlar içinse donanımı URL'ler üzerinden paylaşılabilir ve saniyeler içinde test edilebilir kılan evrensel bir web oyun alanıdır.
