# NetLang Ngspice runtime

Bu dizin NetLang'in yerel Windows simülasyonu için kullandığı, bilinçli olarak
küçültülmüş Ngspice çalışma zamanı paketidir. Ngspice kaynak/test dağıtımı
değildir.

## Sürüm ve kaynak

- Sürüm: **ngspice-46**, Windows x86-64 konsol derlemesi
- Binary'nin raporladığı oluşturma tarihi: 29 Mart 2026
- Resmî indirme sayfası: <https://ngspice.sourceforge.io/download.html>
- Resmî dokümantasyon: <https://ngspice.sourceforge.io/docs.html>
- Resmî geliştirme ve lisans özeti: <https://ngspice.sourceforge.io/devel.html>

Sürüm, `bin/ngspice_con.exe --version` çıktısıyla doğrulanır. Mevcut NetLang
runner yalnız Windows sidecar'ını keşfeder; Linux/macOS paketleme veya sistem
Ngspice discovery henüz desteklenmez.

## Takip edilen runtime profili

| Dosya | Amaç |
| --- | --- |
| `bin/ngspice_con.exe` | NetLang'in batch modunda çağırdığı konsol simulator |
| `bin/libomp140.x86_64.dll` | Bu Windows binary'sinin OpenMP çalışma zamanı |
| `share/ngspice/scripts/spinit` | Deterministik, minimal başlangıç ayarları |
| `docs/COPYING` | Upstream lisans metinleri ve istisnaları |
| `docs/AUTHORS` | Upstream attribution kaydı |
| `docs/README` | Upstream proje ve kaynak bilgisi |

GUI executable, upstream örnek/test ağacı, PDF manual, geliştirme notları,
XSPICE `.cm` code model'leri ve OpenVAF/OSDI model kütüphaneleri NetLang'in
mevcut analog runtime profilinin parçası değildir. `spinit` bu eksik opsiyonel
kütüphaneleri yüklemeye çalışmayacak şekilde açıkça yapılandırılmıştır. Bu
özelliklerden biri ürün kapsamına alındığında, fixture ve dağıtım incelemesiyle
ayrı bir runtime profili tanımlanmalıdır.

## Doğrulama kapsamı

8 Ağustos 2026 tarihinde aşağıdakiler doğrulandı:

1. Yalnız yukarıdaki runtime dosyalarını içeren temiz bir geçici dizinde
   `ngspice_con.exe --version` sürüm 46 raporladı.
2. `examples/test_features.nl` kaynağından üretilen SPICE netlist batch modunda
   exit code 0 ile çalıştı.
3. Ölçüm çıktısı `max_v_my_signal = 6.20001e-08` değerini üretti ve eksik init,
   code-model veya OSDI dosyası hatası vermedi.

Bu smoke doğrulaması desteklenen bütün Ngspice özelliklerini garanti etmez.
Phase 3 integration fixture'ları OP, transient ve AC davranışını ayrıca
kilitleyecektir.

## Lisans ve dağıtım kapısı

`docs/COPYING`, upstream paketin Modified BSD temel lisansını ve KLU, OSDI,
XSPICE gibi bileşenlere ait istisnaları birlikte içerir. Dağıtılan binary
`--version` çıktısında KLU solver ile derlendiğini bildirir; bu nedenle yalnız
ana proje lisansına bakılarak dağıtım kararı verilmemelidir. Lisans dosyası ve
attribution kayıtları binary ile birlikte korunmalıdır.

Bu belge hukuki görüş değildir. Public release/paketleme öncesinde tam binary
provenance'i, karşılık gelen kaynak erişimi ve üçüncü taraf lisans
yükümlülükleri release checklist'inde ayrıca incelenmelidir.

## Yükseltme prosedürü

1. Binary'yi yalnız resmî Ngspice indirme sayfasından al.
2. `--version` çıktısını ve hedef mimariyi kaydet.
3. `docs/COPYING`, `docs/AUTHORS` ve upstream README'yi aynı dağıtımdan yenile.
4. Minimal dosya setini temiz geçici dizinde doğrula.
5. Phase 3 simulator integration fixture'larını çalıştır.
6. Bu belgeyi ve roadmap kanıtını güncelle.
