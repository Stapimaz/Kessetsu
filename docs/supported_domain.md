# Kessetsu İlk Public Sürüm Destek Matrisi

Bu belge Faz 4 için ilan edilen elektriksel kapsamı dondurur. “Destekleniyor”, syntax'ın parse edilmesinden fazlasıdır: typed IR, canonical SPICE, ERC ve ilgili native/Web doğrulama kapılarının bulunması demektir. Tabloda olmayan özellikler fail-closed diagnostic üretmeli; yaklaşık destek varmış gibi sunulmamalıdır.

## Component ve source kapsamı

| Aile | Destek | Canonical pinler | Sınır |
|---|---|---|---|
| Resistor, capacitor, inductor | Destekleniyor | `p1`, `p2` | İdeal lumped eleman; tolerance/temperature/parasitic modeli yok |
| Diode | Destekleniyor | `p1`, `p2` | Builtin veya typed whitelist model |
| BJT NPN/PNP | Destekleniyor | `c`, `b`, `e` | Üç terminalli model; substrate/thermal pin yok |
| MOSFET NMOS/PMOS | Destekleniyor | `d`, `g`, `s` | Üç terminalli model; body ayrı pin değil |
| Op-amp | Destekleniyor | `in_p`, `in_n`, `vcc`, `vee`, `out` | Güvenli canonical subcircuit template; arbitrary subcircuit yok |
| Voltage/current source | Destekleniyor | `plus`, `minus` | DC, `sine`, `pulse`, `ac`, `sine_ac` typed waveform'ları |
| Module port | Flattening iç öğesi | Module tanımına bağlı | Public fiziksel component değildir |

## Model kapsamı

- Builtin: `2N3904`, `2N3906`, `2N2222`, `KESSETSU_POWER_NPN_V1`, `KESSETSU_POWER_PNP_V1`, `1N4148`, `1N4007`, `IRF540`, `KESSETSU_PMOS_V1`, `KESSETSU_OPAMP_V1`.
- Verified generic PMOS: `KESSETSU_PMOS_V1@1.0.1`; portable Ngspice `MOS1` DC modeli, üretici/datasheet veya parasitic model iddiası yok.
- User model: typed diode/BJT/MOSFET parametre whitelist'i.
- User subcircuit: yalnız typed op-amp template'i.
- Package import: exact ad+sürüm, content hash, lisans ve simulator capability içeren `kessetsu.models.v1`/`kessetsu.lock.v1`.
- Desteklenmez: raw `.include`, `.model`, `.subckt`, `.control`; floating package version; arbitrary vendor script/model injection.

## Analysis, dataset ve ölçüm kapsamı

| Alan | Destekleniyor | Açık sınır |
|---|---|---|
| Analysis | OP, transient, AC decade/linear/octave, independent voltage/current DC sweep | Noise, Monte Carlo, sensitivity, temperature sweep yok |
| Dataset | OP scalar; transient/DC real series; AC complex series | Simulator raw format public sözleşme değildir |
| Primitive | `V(net/device)`, `I(device)`, `P(device)` | Safe op-amp internal/output branch current fail-closed |
| Reduction | `value`, `min`, `max`, absolute `peak`, `average`, `rms` | Kompleks AC üzerinde real reduction yok |
| Derived | gain, bandwidth/cutoff, frequency, phase, output power, efficiency, THD, clipping, dissipation | Yalnız belgelenmiş analysis/signal koşullarında |
| Assertion | `<`, `>`, `==`, `<=`, `>=`; PASS/FAIL/ERROR/SKIPPED | Eksik data asla `0`/PASS sayılmaz |

## İlk yayın doğrulama devreleri

- RC low-pass: ilk dikey Web/CLI parity yolu.
- Op-amp gain stage: feedback, AC bandwidth ve transient clipping.
- Dört katlı 8 Ω power amplifier: ana ürün eval'i; gain, yaklaşık 2 W output, THD, clipping, stress ve dissipation.
- Şema corpus'u ayrıca minimal, Wheatstone bridge ve yüksek fan-out topolojilerini kapsar.

## Bilinçli fiziksel sınırlar

Kessetsu'nun ilk sürümü schematic-level SPICE mühendislik aracıdır; PCB layout/DRC, transmission-line/EM field çözümü, RF S-parameter workflow'u, digital HDL, thermal/aging/reliability, package/PCB parasitic extraction, EMC/ESD, manufacturing tolerance/Monte Carlo ve datasheet limit database'i sağlamaz. Simülasyon sonucu gerçek laboratuvar ölçümü veya mühendis incelemesinin yerine geçmez. Desteklenen analog/mixed-signal SPICE kapsamı ileride typed domain sözleşmeleriyle genişleyebilir; bugünkü mimari eğitim devreleriyle sınırlı değildir.
