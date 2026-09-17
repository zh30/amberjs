import { useEffect, useLayoutEffect } from "react";
import { useLocation, useNavigationType } from "react-router-dom";

const positions = new Map<string, number>();

function scrollToY(top: number) {
  window.scrollTo(0, top);
}

function scrollToHash(hash: string) {
  const id = decodeURIComponent(hash.replace(/^#/, ""));
  if (!id) return false;
  const el = document.getElementById(id);
  if (!el) return false;
  el.scrollIntoView();
  return true;
}

/** New navigations start at the top; back/forward restore the previous page. */
export function RouteScroll() {
  const location = useLocation();
  const navType = useNavigationType();

  useEffect(() => {
    if ("scrollRestoration" in history) {
      history.scrollRestoration = "manual";
    }
  }, []);

  useEffect(() => {
    const key = location.key;
    const save = () => {
      positions.set(key, window.scrollY);
    };
    window.addEventListener("scroll", save, { passive: true });
    return () => {
      save();
      window.removeEventListener("scroll", save);
    };
  }, [location.key]);

  useLayoutEffect(() => {
    const apply = () => {
      if (navType === "POP") {
        scrollToY(positions.get(location.key) ?? 0);
        return;
      }
      if (location.hash && scrollToHash(location.hash)) return;
      scrollToY(0);
    };

    apply();
    const frame = window.requestAnimationFrame(apply);
    return () => window.cancelAnimationFrame(frame);
  }, [location.hash, location.key, location.pathname, navType]);

  return null;
}
