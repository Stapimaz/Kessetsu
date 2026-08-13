// Two-input inverting summing amplifier.
net GND
net VCC
net VEE
net IN1
net IN2
net SUM
net OUT
source VP 12V
source VN 12V
source V1 sine(0V,100mV,1kHz)
source V2 sine(0V,50mV,2kHz)
opamp U1 NLANG_OPAMP_V1
resistor R1 10k
resistor R2 20k
resistor RF 47k
resistor RL 10k
connect VP.minus, VN.plus, V1.minus, V2.minus, U1.in_p, RL.p2 to GND
connect VP.plus, U1.vcc to VCC
connect VN.minus, U1.vee to VEE
connect V1.plus, R1.p1 to IN1
connect V2.plus, R2.p1 to IN2
connect R1.p2, R2.p2, RF.p2, U1.in_n to SUM
connect U1.out, RF.p1, RL.p1 to OUT
simulate op
