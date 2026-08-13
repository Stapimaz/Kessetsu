// Inverting op-amp stage: explicit summing node and local feedback loop.
net GND
net VCC
net VEE
net IN
net SUM
net OUT
source VP 12V
source VN 12V
source VIN sine_ac(0V,100mV,1kHz,1V)
opamp U1 NLANG_OPAMP_V1
resistor RIN 10k
resistor RF 47k
resistor RL 10k
connect VP.minus to GND
connect VP.plus to VCC
connect VN.plus to GND
connect VN.minus to VEE
connect VIN.minus to GND
connect VIN.plus to IN
connect RIN.p1 to IN
connect RIN.p2 to SUM
connect U1.in_p to GND
connect U1.in_n to SUM
connect U1.vcc to VCC
connect U1.vee to VEE
connect U1.out to OUT
connect RF.p1 to OUT
connect RF.p2 to SUM
connect RL.p1 to OUT
connect RL.p2 to GND
simulate op

