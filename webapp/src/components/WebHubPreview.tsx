import { Activity, Check, CircuitBoard, Code2 } from 'lucide-react';

export function WebHubPreview() {
  return (
    <figure className="hub-preview" aria-label="Kessetsu Web Hub product preview">
      <div className="preview-header">
        <span><CircuitBoard size={13} /> KESSETSU</span>
        <small>Connectivity verified</small>
      </div>
      <div className="preview-workspace">
        <section className="preview-source" aria-label="Circuit source preview">
          <strong><Code2 size={13} /> Source</strong>
          <pre>{`net GND
net IN
net OUT

source VIN ac(1V)
resistor R1 1k
capacitor C1 159.154943nF

connect VIN.plus to IN
connect R1.p1 to IN
connect R1.p2, C1.p1 to OUT
connect VIN.minus, C1.p2 to GND

simulate ac dec 40 100Hz 100kHz
assert cutoff(V(OUT),V(IN)) > 990Hz`}</pre>
        </section>
        <div className="preview-output">
          <section className="preview-schematic" aria-label="Verified RC schematic preview">
            <strong><CircuitBoard size={13} /> Schematic</strong>
            <svg viewBox="0 0 430 210" role="img" aria-label="RC low-pass schematic">
              <g className="preview-wires">
                <path d="M64 76H142 M226 76H326 M326 76V120 M64 76V92 M64 132V166 M326 138V166" />
              </g>
              <g className="preview-symbols">
                <circle cx="64" cy="112" r="20" />
                <path d="M58 105H70 M64 99V111 M58 120H70" />
                <path d="M142 76L153 64L166 88L179 64L192 88L205 64L216 76H226" />
                <path d="M306 120H346 M306 138H346" />
                <path d="M326 166V176 M310 176H342 M315 183H337 M321 190H331" />
                <path d="M64 166V176 M48 176H80 M53 183H75 M59 190H69" />
              </g>
              <g className="preview-junctions"><circle cx="326" cy="76" r="4" /></g>
              <g className="preview-labels">
                <text x="51" y="68">IN</text>
                <text x="169" y="48">R1</text>
                <text x="164" y="106">1 kΩ</text>
                <text x="334" y="68">OUT</text>
                <text x="354" y="126">C1</text>
                <text x="354" y="144">159 nF</text>
                <text x="34" y="116">VIN</text>
              </g>
            </svg>
          </section>
          <section className="preview-results" aria-label="Simulation result preview">
            <strong><Activity size={13} /> Results</strong>
            <div>
              <span><Check size={12} /> 5 / 5 requirements passed</span>
              <code>cutoff = 1.000 kHz</code>
            </div>
          </section>
        </div>
      </div>
      <figcaption>One source. One Core. The same verified result in Web and CLI.</figcaption>
    </figure>
  );
}
