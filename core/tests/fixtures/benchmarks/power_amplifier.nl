// Four-stage amplifier: input buffer, voltage gain, error/driver and class-B output.
// Target: approximately 2 W RMS into 8 ohm from a 100 mV-peak, 1 kHz input.
net GND
net VCC
net VEE
net IN
net BUF
net GAIN
net FB
net DRIVE
net OUT
source VP 12V
source VN 12V
source VIN sine_ac(0V,100mV,1kHz,1V)
opamp U1 NLANG_OPAMP_V1
opamp U2 NLANG_OPAMP_V1
opamp U3 NLANG_OPAMP_V1
resistor RF 55.6k
resistor RG 1k
transistor QN npn NLANG_POWER_NPN_V1
transistor QP pnp NLANG_POWER_PNP_V1
resistor RL 8
connect VP.minus to GND
connect VP.plus to VCC
connect VN.plus to GND
connect VN.minus to VEE
connect VIN.minus to GND
connect VIN.plus to IN
connect U1.in_p to IN
connect U1.in_n to BUF
connect U1.vcc to VCC
connect U1.vee to VEE
connect U1.out to BUF
connect U2.in_p to BUF
connect U2.in_n to FB
connect U2.vcc to VCC
connect U2.vee to VEE
connect U2.out to GAIN
connect RF.p1 to GAIN
connect RF.p2 to FB
connect RG.p1 to FB
connect RG.p2 to GND
connect U3.in_p to GAIN
connect U3.in_n to OUT
connect U3.vcc to VCC
connect U3.vee to VEE
connect U3.out to DRIVE
connect QN.c to VCC
connect QN.b to DRIVE
connect QN.e to OUT
connect QP.c to VEE
connect QP.b to DRIVE
connect QP.e to OUT
connect RL.p1 to OUT
connect RL.p2 to GND
simulate ac dec 30 10Hz 10MHz
simulate tran 5us 10ms
assert gain(V(OUT),V(IN)) > 55
assert gain(V(OUT),V(IN)) < 58
assert output_power(V(OUT),RL,2ms,10ms) > 1.9W
assert output_power(V(OUT),RL,2ms,10ms) < 2.1W
assert efficiency(V(OUT),RL,V(VCC),I(VP),V(VEE),I(VN),2ms,10ms) > 35%
assert thd(V(OUT),1kHz,2ms,10ms,hann) < 3%
assert clipping(V(OUT),-10V,10V) < 0.1%
assert dissipation(QN,2ms,10ms) < 2W
assert dissipation(QP,2ms,10ms) < 2W
assert peak(V(QN.c,QN.e),2ms,10ms) < 20V
assert peak(I(QN)) < 1A
assert peak(P(QN)) < 5W
