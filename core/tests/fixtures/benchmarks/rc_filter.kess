// Canonical first-order RC low-pass: 1 V AC input, fc ~= 1 kHz.
net GND
net IN
net OUT
source VIN ac(1V)
resistor R1 1k
capacitor C1 159.154943nF
connect VIN.minus to GND
connect VIN.plus to IN
connect R1.p1 to IN
connect R1.p2 to OUT
connect C1.p1 to OUT
connect C1.p2 to GND
simulate ac dec 40 10Hz 100kHz
assert gain(V(OUT),V(IN)) > 0.99
assert cutoff(V(OUT),V(IN)) > 990Hz
assert cutoff(V(OUT),V(IN)) < 1010Hz
assert phase(V(OUT),V(IN),1kHz) > -46deg
assert phase(V(OUT),V(IN),1kHz) < -44deg
