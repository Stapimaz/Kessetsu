// Bidirectional diode clamp around a loaded signal node.
net GND
net VCC
net IN
net OUT
source VCCS 5V
source VIN sine(0V,8V,1kHz)
resistor RIN 1k
resistor RL 10k
diode DHI
diode DLO
connect VCCS.minus, VIN.minus, RL.p2 to GND
connect VCCS.plus, DHI.p2 to VCC
connect VIN.plus, RIN.p1 to IN
connect RIN.p2, RL.p1, DHI.p1, DLO.p2 to OUT
connect DLO.p1 to GND
simulate tran 10us 5ms

