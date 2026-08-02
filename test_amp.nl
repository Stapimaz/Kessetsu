module VoltageDivider(in, out, gnd) {
    resistor R1 10k
    resistor R2 20k
    connect in R1.p1
    connect R1.p2 out
    connect R1.p2 R2.p1
    connect R2.p2 gnd
}

battery B1 9V
use VoltageDivider myDiv
transistor Q1 NPN
resistor R3 1k

// Floating pin hatasi olmasi icin Q1.c yi bilerek bos birakiyoruz
connect B1.plus myDiv.in
connect B1.minus myDiv.gnd

connect myDiv.out Q1.b
connect Q1.e B1.minus
