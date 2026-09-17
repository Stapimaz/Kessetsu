# Threshold memristor example notice

Origin: [Ngspice example](https://github.com/imr/ngspice/blob/master/examples/memristor/memristor.sp),
retrieved 2026-09-17. Model authors: Y. V. Pershin and M. Di Ventra;
example/parameter selection: Holger Vogt, 2012.
Paper: [SPICE model of memristive devices with threshold](https://arxiv.org/abs/1204.2600).

`memristor.lib` extracts the subcircuit only, retaining its equations and artificial
coefficients. The example's external `stime=10n` becomes a subcircuit default; the
top-level source, plotting/control commands and sequential frequency changes are not
included. Use transient `uic` to apply the model's capacitor initial condition.
This is a simulation demonstration, not calibration against a fabricated memristor.

The upstream [license](https://github.com/imr/ngspice/blob/master/COPYING) applies
Modified BSD to source, tests and examples except its enumerated exceptions; this
example is not in that exception list. Its notice and terms are retained below.
This file is third-party BSD-3-Clause content, not relicensed by Kessetsu's AGPL license.

Copyright 1985 - 2018, Regents of the University of California and others

Redistribution and use in source and binary forms, with or without modification,
are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice,
   this list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.
3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from this
   software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDERS OR CONTRIBUTORS BE
LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
POSSIBILITY OF SUCH DAMAGE.
