// Symmetric BJT differential pair with a shared tail and collector loads.
net GND
net VCC
net INP
net INN
net TAIL
net OUTP
net OUTN
source VCCS 12V
source VINP 1V
source VINN 1V
current_source ITAIL 2mA
transistor Q1 npn NLANG_POWER_NPN_V1
transistor Q2 npn NLANG_POWER_NPN_V1
resistor RC1 4.7k
resistor RC2 4.7k
connect VCCS.minus, VINP.minus, VINN.minus, ITAIL.minus to GND
connect VCCS.plus, RC1.p1, RC2.p1 to VCC
connect VINP.plus, Q1.b to INP
connect VINN.plus, Q2.b to INN
connect Q1.e, Q2.e, ITAIL.plus to TAIL
connect RC1.p2, Q1.c to OUTP
connect RC2.p2, Q2.c to OUTN
simulate op

