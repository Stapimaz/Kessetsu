// Mixed-orientation passive ladder for series flow and shunt branch layout.
net GND
net IN
net N1
net N2
net OUT
source VIN ac(1V)
resistor RS 50
inductor L1 10mH
capacitor C1 100nF
resistor R2 100
capacitor C2 220nF
resistor RL 1k
connect VIN.minus, C1.p2, C2.p2, RL.p2 to GND
connect VIN.plus, RS.p1 to IN
connect RS.p2, L1.p1 to N1
connect L1.p2, C1.p1, R2.p1 to N2
connect R2.p2, C2.p1, RL.p1 to OUT
simulate ac dec 30 10Hz 1MHz
