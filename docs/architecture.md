# NetLang Projesi Mimari Anayasası

Bu doküman, NetLang projesinin çekirdek algoritmalarını, derleme süreçlerini ve dil tasarım standartlarını açıklar. **Projeye dahil olan tüm geliştiriciler ve Yapay Zeka Ajanları (AI Agents), projede herhangi bir kod yazmadan önce bu dokümandaki kurallara uymak ZORUNDADIR.**

## 1. Sistemin Temel Parçaları

NetLang projesi temelde iki ana parçadan oluşur:
1. **`netlang-core` (Rust):** Dilin ayrıştırıcısı (parser), AST oluşturucusu, SPICE netlist jeneratörü ve otomatik şema (layout) motorunu barındıran çekirdek kütüphane.
   - Hem bir kütüphane (`lib`), hem CLI (`bin.rs`) olarak hem de Web için WASM (`wasm.rs`) olarak derlenir.
2. **`webapp` (React/TS):** Kullanıcının kodu yazdığı ve sonuçları (şema/grafik) gördüğü UI. `netlang-core`'u WASM üzerinden tarayıcı içinde gerçek zamanlı çalıştırır.

## 2. Dilin Sözdizimi (Syntax) ve Kurallar
- Desteklenen Temel Parçalar: `resistor`, `capacitor`, `inductor`, `diode`, `transistor`, `mosfet`, `opamp`, `source`.
- **ÖNEMLİ KURAL:** Piller veya AC/DC tüm sinyal jeneratörleri için `battery` KELİMESİ KULLANILMAZ. Yerine **`source`** kullanılır. SPICE'ta hepsi `V_` ile ifade edilir.
- **Değerler:** Bileşen değerleri sayı/birim olabileceği gibi çift tırnaklı metin de (`String`) olabilir.
  - *Doğru:* `source Vin "SINE(0 1V 1kHz)"`
  - *Doğru:* `resistor R1 10k`
- **Simülasyon Komutları:** `simulate <cmd> <args*>`
  - *Doğru:* `simulate tran "10us" "1ms"`

## 3. Düğüm (Node) İsimlendirme Algoritması (`graph.rs`)
SPICE motoru için düğüm numaraları rastgele tam sayılar DEĞİLDİR. Şema çıktılarının ve simülasyon grafiklerinin okunabilir olması için özel bir algoritma kullanılır:
1. `GND` hattına bağlı her şey (örn: `Vin.minus`) her zaman `"0"` düğümündedir.
2. Diğer tüm düğümler (Nets), kendisine bağlı olan pinlerin bir listesini tutar.
3. Bu pin listesi **alfabetik olarak sıralanır** ve en baştaki pinin adı alınır.
4. Pin adındaki nokta `.` karakteri alt çizgiye `_` çevrilip başına `N_` eklenir.
   - *Örnek:* Bir düğüme `R1.p2`, `C1.p1` ve `Q1.b` bağlıysa. Alfabetik sırada ilk gelen `C1.p1`'dir. Düğüm ismi **`N_C1_p1`** olur. SPICE çıktısında voltaj `v(N_C1_p1)` olarak okunur.
   - **KURAL:** Bu algoritma ASLA BOZULMAMALIDIR. Gelecekte Memristör bile eklense, alfabetik sıralama tüm parçalar için stabil bir isimlendirme garantisi sunar.

## 4. SPICE Motoru ve Standart Modeller
Ngspice entegrasyonu tamamen gömülü çalışır.
- Eğer AST içerisinde `2N3904`, `1N4148`, `IRF540` gibi bilinen bir parça kullanılırsa, `graph.rs` bu parçanın fiziksel SPICE `.model` veya `.subckt` tanımını otomatik olarak netlist'in sonuna enjekte eder. Kullanıcıların `.model` yazmasına gerek yoktur.

## 5. Şema (Layout) Motoru (`layout.rs`)
Şematiği çizerken parçaları x/y koordinatlarına yerleştirmek için **DFS (Depth-First Search)** tabanlı bir grafik tarama algoritması kullanılır.
- **KURAL:** Algoritma "Yön Farkındalığına (Orientation Awareness)" sahiptir.
- GND pinlerine doğru olan yollar Her Zaman AŞAĞI yönlendirilir.
- Çıkış (Output) pinleri SAĞA, Giriş (Input) pinleri SOLA bakmalıdır.
- Sinyal akışında `is_signal_pin` fonksiyonu kullanılarak yollar önceliklendirilir. (Örneğin BJT'de `b` pini sinyal girişi sayılır, Akım `c` den `e` ye akar).

## 6. NetLang Vizyonu ve Ekosistem Manifestosu

NetLang, elektronik devre tasarımını **"Sıfır Sürtünme" (Zero-Friction)** felsefesiyle metne döken, bağımsız, ultra hızlı ve hepsi bir arada bir **"Donanım Olarak Kod" (Hardware-as-Code)** ekosistemidir.

Vizyonumuz, hantal IDE'leri ve karmaşık kurulum süreçlerini ortadan kaldırarak; yapay zeka ajanlarına doğrudan terminalden kendi kendilerine devre tasarlatmak ve insanlara donanımı CodePen veya iLovePDF sadeliğinde sunmaktır. Bu ekosistem iki ana sütun üzerinde yükselir:

### 1. NetLang CLI (Yapay Zeka ve Geliştiriciler İçin Motor)
Hiçbir dış kurulum veya internet bağlantısı gerektirmeyen, içine endüstri standardı simülasyon fizik motoru (Ngspice) kalıcı olarak gömülmüş tek parça bir Rust derleyicisidir.

- **Dağıtım Yolları:** Web sitesinden doğrudan indirilebilen tek bir `.exe` dosyası, terminal üzerinden `cargo install` ile kurma veya ileride bir VS Code eklentisinin arkasında sessizce çalışan görünmez bir işçi (worker).
- **Kullanım Senaryosu ve TDD (Test Odaklı Tasarım):** Bir AI ajanı (veya donanım mühendisi), terminali açıp motoru çalıştırır. NetLang devreyi derler, kısa devreleri veya boşta kalan pinleri (DRC) insancıl ve yapay zekanın anında çözümleyebileceği netlikte fırlatır. Otomatik olarak akım/voltaj simülasyonlarını yapar. Ajan, SPICE'ın karmaşık matematiğinde boğulmadan terminalden gelen bu net çıktıları okuyarak, tıpkı yazılımda Unit Test (Birim Testi) yazar gibi (TDD - Test-Driven Development) devreyi kendi kendine optimize eder (Self-Healing). Her şey saniyeler içinde, sadece komut satırında biter.

### 2. NetLang Web Hub (İnsanlar İçin Vitrin ve Oyun Alanı)
Kullanıcıların siteye girdiği an kayıtsız, şartsız, indirmesiz kullanabildiği; arka planda Rust çekirdeğini WASM ile doğrudan tarayıcının yerel gücünü kullanarak çalıştıran arayüzdür.

- **Hızlı Prototip Testi (Playground):** Masanda yeni bir mikrodenetleyici veya röle var. Fiziksel devreyi kurmadan hemen önce, "Acaba bu bağlantı işlemciyi yakar mı?" diye hızlıca test etmek istiyorsun. Siteye girer, üç satır NetLang kodu yazar, anında simülasyon ve DRC onayını alır ve güvenle masana dönersin.
- **Anında Paylaşım (CodePen Modeli):** Donanım dünyasında ekran görüntüsü veya kağıt çizimi devrini bitiren özelliktir. Kod, projenin URL'sine veya kısa bir ID'ye gömülür. Bir forumda devreni paylaştığında, linke tıklayan herkes o devrenin hatasız profesyonel SVG şemasını, canlı kodunu ve simülasyonunu anında kendi tarayıcısında çalıştırabilir.
- **API Vitrini ve Satış Noktası:** Kurumsal şirketlere veya kendi ajanlarına donanım tasarlatmak isteyen teknoloji devlerine satacağımız altyapının şov alanıdır. Ana sayfadaki hareketli ve anında şema çizen canlı demo, motorun hızını kanıtlayan en büyük pazarlama aracıdır.

**ÖZETLE:** NetLang bir "çizim programı" değil, bir altyapıdır. Geliştiriciler ve Yapay Zeka için terminalde otonom çalışan bir simülasyon/derleme motoru; insanlar içinse donanımı URL'ler üzerinden paylaşılabilir ve saniyeler içinde test edilebilir kılan evrensel bir web oyun alanıdır.
