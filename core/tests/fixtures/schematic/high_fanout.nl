// Characterization fixture: one named signal fans out to eight loads.
net GND
net BUS
source V1 5V
resistor R1 1k
resistor R2 2k
resistor R3 3k
resistor R4 4k
resistor R5 5k
resistor R6 6k
resistor R7 7k
resistor R8 8k
connect V1.minus to GND
connect V1.plus to BUS
connect R1.p1, R2.p1, R3.p1, R4.p1, R5.p1, R6.p1, R7.p1, R8.p1 to BUS
connect R1.p2, R2.p2, R3.p2, R4.p2, R5.p2, R6.p2, R7.p2, R8.p2 to GND
simulate op
