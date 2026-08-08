// Phase 2 Feature Edge-case Test

net my_power
net my_signal
net my_ground

source V1 10V
current_source I1 sine(0mA, 5mA, 10kHz)

// Explicit polarity hints
transistor Q1 pnp 2N3906
transistor Q2 npn

resistor R1 10k
resistor R2 5k

capacitor C1 100uF

// Multi-pin connections
connect V1.plus, I1.plus to my_power
connect V1.minus, I1.minus to my_ground

// More connections
connect R1.p1, Q1.e to my_power
connect R1.p2, Q1.b to C1.p1
connect C1.p2 to my_signal

connect Q1.c, Q2.b to R2.p1
connect R2.p2, Q2.e to my_ground
connect Q2.c to my_signal

// Assertions
assert max(V(my_signal)) < 5V
assert peak(I(V1)) < 100mA

simulate op
