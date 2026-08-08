// Ortak Emiter (Common Emitter) BJT Amplifikatör Devresi
// LLM Agent Tarafindan Otonom Olarak Duzeltilmis Versiyon

source Vcc 12V

// Base gerilim bolucu biasing
resistor R1 10k
resistor R2 2.2k

// Collector ve Emitter direncleri
resistor Rc 1k
resistor Re 100

// BJT Transistor
transistor Q1 2N3904

// Giris/Cikis ve Bypass Kapasitorleri
capacitor Cin 10uF
capacitor Cout 10uF
capacitor Ce 100uF

// --- TEST ICIN EKSTRA ELEMANLAR (Floating pınleri baglamak icin) ---
// Sinyal Kaynagi (Giris) ve Yuk Direnci (Cikis)
// NetLang'da henuz AC source yok, bu yuzden DC batarya kullaniyoruz (ileride eklenecek)
source Vin 1V
resistor Rload 10k

// --- BAGLANTILAR ---

// Guc hatti (Vcc) baglantilari
connect Vcc.plus to R1.p1
connect Vcc.plus to Rc.p1

// Toprak hatti (GND) baglantilari
connect Vcc.minus to R2.p2
connect Vcc.minus to Re.p2
connect Vcc.minus to Ce.p2
connect Vcc.minus to Vin.minus
connect Vcc.minus to Rload.p2

// Base biasing dugumu
connect R1.p2 to R2.p1
connect R1.p2 to Q1.b
connect R1.p2 to Cin.p2

// Giris sinyali hatti
connect Vin.plus to Cin.p1 

// Collector dugumu
connect Rc.p2 to Q1.c
connect Rc.p2 to Cout.p1

// Cikis sinyali hatti
connect Cout.p2 to Rload.p1

// [FIXED] Emitter dugumu baglantilari
connect Q1.e to Re.p1
connect Q1.e to Ce.p1

simulate op
