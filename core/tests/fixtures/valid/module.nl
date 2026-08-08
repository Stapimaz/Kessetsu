module Divider(in, out, gnd) {
    resistor R1 10k
    resistor R2 20k
    connect in to R1.p1
    connect R1.p2 to out
    connect R1.p2 to R2.p1
    connect R2.p2 to gnd
}

source V1 9V
use Divider divider
connect V1.plus to divider.in
connect V1.minus to divider.gnd
