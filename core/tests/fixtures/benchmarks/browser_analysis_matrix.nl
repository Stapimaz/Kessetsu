// Browser runtime contract fixture covering every Phase 4 analysis dataset.
net GND
net IN
net OUT
source VIN sine_ac(0V,1V,1kHz,1V)
resistor R1 1k
capacitor C1 159.154943nF
connect VIN.minus to GND
connect VIN.plus to IN
connect R1.p1 to IN
connect R1.p2 to OUT
connect C1.p1 to OUT
connect C1.p2 to GND
simulate op
simulate tran 10us 2ms
simulate ac dec 20 10Hz 100kHz
simulate dc VIN -1V 1V 100mV
assert value(V(IN)) == 0V
assert max(V(OUT)) < 1.1V
assert gain(V(OUT),V(IN)) > 0.7
assert cutoff(V(OUT),V(IN)) > 990Hz
assert cutoff(V(OUT),V(IN)) < 1010Hz
