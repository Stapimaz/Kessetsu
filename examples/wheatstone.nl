source V1 10V
resistor R1 10k
resistor R2 20k
resistor R3 10k
resistor R4 10k
resistor Rx 5k

// Top Rail
connect V1.plus to R1.p1
connect V1.plus to R3.p1

// Mid nodes
connect R1.p2 to R2.p1
connect R3.p2 to R4.p1

// Bottom Rail (Ground)
connect R2.p2 to V1.minus
connect R4.p2 to V1.minus

// Bridge connection
connect R1.p2 to Rx.p1
connect R3.p2 to Rx.p2

// DC Operating Point Analysis
simulate op
