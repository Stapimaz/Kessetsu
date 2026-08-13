// Common-source MOSFET stage with drain resistor and source degeneration.
net GND
net VDD
net IN
net OUT
net SOURCE
source VDD1 12V
source VIN 2V
mosfet M1 IRF540
resistor RD 2.2k
resistor RS 220
resistor RL 10k
connect VDD1.minus, VIN.minus to GND
connect VDD1.plus, RD.p1 to VDD
connect VIN.plus, M1.g to IN
connect RD.p2, M1.d, RL.p1 to OUT
connect M1.s, RS.p1 to SOURCE
connect RS.p2, RL.p2 to GND
simulate op
