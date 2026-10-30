import { useEffect, useRef, useState, type CSSProperties, type RefObject } from "react";
import { Mast } from "../components/Mast";
import { CommandLine, CopyInstall, InstallTabs } from "../components/InstallBar";
import { useLang } from "../lib/i18n";
import { fillCount, landingCopy } from "../lib/landing-copy";
import { CONFORMANCE, CONFORMANCE_URL, REPO_URL, SCOPE_DOC_URL } from "../lib/scope";
import { AMBER_VERSION } from "../lib/version";

const INK_CELLS = new Set([2, 3, 6, 7]);
const SCOPE_MARKER = "docs/CURRENT_SCOPE.md";

function motionDelay(ms: number): CSSProperties {
  return { ["--d"]: `${ms}ms` } as CSSProperties;
}

function useSettle<T extends HTMLElement>(): { ref: RefObject<T | null>; on: boolean } {
  const ref = useRef<T>(null);
  const [on, setOn] = useState(false);

  useEffect(() => {
    const node = ref.current;
    if (!node) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setOn(true);
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        const entry = entries[0];
        if (!entry?.isIntersecting) return;
        const viewport = entry.rootBounds?.height ?? window.innerHeight;
        const target = entry.boundingClientRect.height;
        const visible = entry.intersectionRect.height;
        // A section taller than the screen can never be 30% in view.
        if (visible < Math.min(target, viewport) * 0.3) return;
        setOn(true);
        observer.disconnect();
      },
      { threshold: [0, 0.15, 0.3, 0.6] },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, []);

  return { ref, on };
}

function FaqAnswer({ text }: { text: string }) {
  const index = text.indexOf(SCOPE_MARKER);
  if (index < 0) return <p>{text}</p>;
  return (
    <p>
      {text.slice(0, index)}
      <a href={SCOPE_DOC_URL}>{SCOPE_MARKER}</a>
      {text.slice(index + SCOPE_MARKER.length)}
    </p>
  );
}

function Tagline({
  line1,
  line2a,
  line2b,
}: {
  line1: string;
  line2a: string;
  line2b: string;
}) {
  const ref = useRef<HTMLElement>(null);
  const [on, setOn] = useState(false);

  useEffect(() => {
    const node = ref.current;
    if (!node) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setOn(true);
      return;
    }
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry?.isIntersecting) setOn(true);
      },
      { threshold: 0.2 },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, []);

  const word = on ? "w on" : "w";
  // Fullwidth punctuation already separates the Chinese line. A forced space
  // reads as a hole: 「先读契约， 再依赖它。」
  const joiner = /[\u3001\u3002\uFF0C]$/.test(line2a) ? "" : " ";
  return (
    <section className="tagline" id="tagline" ref={ref}>
      <p>
        <span className={word}>{line1}</span>
      </p>
      <p>
        <span className={word}>{line2a}</span>
        {joiner}
        <span className={word}>{line2b}</span>
      </p>
    </section>
  );
}

function Proof({
  before,
  mit,
  mid,
  scorecard,
  end,
}: {
  before: string;
  mit: string;
  mid: string;
  scorecard: string;
  end: string;
}) {
  return (
    <p className="proof hero-in" style={motionDelay(260)}>
      {AMBER_VERSION}
      {before}
      <a href={`${REPO_URL}/blob/main/LICENSE`}>{mit}</a>
      {mid}
      <a href={CONFORMANCE_URL}>{scorecard}</a>
      {end}
    </p>
  );
}

export default function HomePage() {
  const { lang } = useLang();
  const text = landingCopy[lang];
  const scorecard = fillCount(text.scorecard, CONFORMANCE.pass, CONFORMANCE.total);
  const stepsView = useSettle<HTMLUListElement>();
  const pairView = useSettle<HTMLElement>();
  const endView = useSettle<HTMLDivElement>();

  return (
    <>
      <div className="hero">
        <Mast appear trailing={<InstallTabs />} />
        <div className="lead">
          <h1 className="hero-in" style={motionDelay(50)}>
            {text.headline}
          </h1>
          <p className="deck hero-in" style={motionDelay(120)}>
            {text.deck}
          </p>
          <p className="sub hero-in" style={motionDelay(190)}>
            {text.sub}
          </p>
          <Proof
            before={text.proofBefore}
            mit={text.mit}
            mid={text.proofMid}
            scorecard={scorecard}
            end={text.proofEnd}
          />
        </div>
        <div className="install">
          <div className="hero-in" style={motionDelay(80)}>
            <CommandLine />
          </div>
          <div className="hero-in" style={motionDelay(160)}>
            <CopyInstall wide />
          </div>
          <div className="receipt hero-in" style={motionDelay(240)}>
            <span>$ amber --version</span>
            <b>{AMBER_VERSION}</b>
            <span>$ amber eval "1 + 1"</span>
            <b>2</b>
          </div>
        </div>
        <ul className="floor">
          {text.floor.map((cell, index) => (
            <li key={cell.title} className={INK_CELLS.has(index) ? "ink" : undefined}>
              <div className="hero-in" style={motionDelay(300 + (index % 4) * 40)}>
                <strong>{cell.title}</strong>
                <span>{cell.body}</span>
              </div>
            </li>
          ))}
        </ul>
      </div>

      <Tagline line1={text.tagline1} line2a={text.tagline2a} line2b={text.tagline2b} />

      <ul ref={stepsView.ref} className={stepsView.on ? "steps settle in" : "steps settle"}>
        {text.steps.map((step, index) => (
          <li key={step.title} className="rise" style={motionDelay(index * 60)}>
            <strong>{step.title}</strong>
            <span>
              {step.a}
              <br />
              {step.b}
            </span>
          </li>
        ))}
      </ul>

      <section ref={pairView.ref} className={pairView.on ? "pair settle in" : "pair settle"}>
        <div className="rise" style={motionDelay(0)}>
          <h2>{text.notTitle}</h2>
          <ul className="no">
            {text.not.map((item) => (
              <li key={item.title}>
                <strong>{item.title}</strong>
                <span>{item.body}</span>
              </li>
            ))}
          </ul>
        </div>
        <div className="rise" style={motionDelay(70)}>
          <h2>{text.faqTitle}</h2>
          {text.faq.map((item) => (
            <details key={item.q} className="faq">
              <summary>{item.q}</summary>
              <FaqAnswer text={item.a} />
            </details>
          ))}
        </div>
      </section>

      <div ref={endView.ref} className={endView.on ? "end settle in" : "end settle"}>
        <div className="rise" style={motionDelay(0)}>
          <CommandLine />
        </div>
        <div className="rise" style={motionDelay(70)}>
          <CopyInstall />
        </div>
      </div>
    </>
  );
}
