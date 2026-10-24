import { render, screen, act } from "@testing-library/react";
import { AuthStatusBadge } from "../Sep10AuthFlow";

// Plain (unpadded) base64, which is what `atob` in the component decodes.
function makeJwt(payload: Record<string, unknown>): string {
  const b64 = (obj: unknown) => btoa(JSON.stringify(obj)).replace(/=+$/, "");
  return `${b64({ alg: "HS256", typ: "JWT" })}.${b64(payload)}.signature`;
}

const wallet = {
  address: "GABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRS",
  network: "testnet",
};

// A fixed "now" so every expectation below is exact arithmetic, not a guess.
const NOW_SEC = 1_800_000_000;

beforeEach(() => {
  jest.useFakeTimers();
  jest.setSystemTime(NOW_SEC * 1000);
});

afterEach(() => {
  jest.useRealTimers();
});

/** The "EXPIRES IN" countdown, formatted as `<h>h <m>m <s>s`. */
function countdown(): string {
  return screen.getByText(/^\d+h \d+m \d+s$/).textContent ?? "";
}

/** The TOKEN VALIDITY percentage label (`—` when no lifetime is knowable). */
function pctLabel(): string {
  return screen.getByText("TOKEN VALIDITY").parentElement?.textContent ?? "";
}

describe("token validity window (#1242)", () => {
  it("treats a freshly issued 24h token as fully valid", () => {
    // expiresIn = exp - now = 86400s, lifetime = 86400s.
    render(<AuthStatusBadge wallet={wallet} jwt={makeJwt({ iat: NOW_SEC, exp: NOW_SEC + 86400 })} />);
    expect(countdown()).toBe("24h 0m 0s");
    expect(pctLabel()).toContain("100.0%");
  });

  it("does not assume a 24h lifetime for a 1h token", () => {
    // The bug: deriving age as `expiry - 24h` made a 1h token read as ~99% elapsed.
    // With the lifetime taken from iat/exp it is simply brand new.
    render(<AuthStatusBadge wallet={wallet} jwt={makeJwt({ iat: NOW_SEC, exp: NOW_SEC + 3600 })} />);
    expect(countdown()).toBe("1h 0m 0s");
    expect(pctLabel()).toContain("100.0%");
  });

  it("does not assume a 24h lifetime for a 7d token", () => {
    // 604800s remaining out of a 604800s lifetime.
    render(
      <AuthStatusBadge wallet={wallet} jwt={makeJwt({ iat: NOW_SEC, exp: NOW_SEC + 604800 })} />,
    );
    expect(countdown()).toBe("168h 0m 0s");
    expect(pctLabel()).toContain("100.0%");
  });

  it("reports the true remaining fraction of a half-elapsed 2h token", () => {
    // Issued 1h ago with a 2h lifetime, so 1h of 2h remains: 50.0%.
    const issued = NOW_SEC - 3600;
    render(<AuthStatusBadge wallet={wallet} jwt={makeJwt({ iat: issued, exp: issued + 7200 })} />);
    expect(countdown()).toBe("1h 0m 0s");
    expect(pctLabel()).toContain("50.0%");
  });

  it("reports the true fraction of a 90-minute token one third through", () => {
    // Issued 30m ago with a 90m lifetime: 60m of 90m remains.
    const issued = NOW_SEC - 1800;
    render(<AuthStatusBadge wallet={wallet} jwt={makeJwt({ iat: issued, exp: issued + 5400 })} />);
    expect(countdown()).toBe("1h 0m 0s");
    // 3600 / 5400 = 66.7%. A hardcoded 24h divisor would have shown ~0.3% here.
    expect(pctLabel()).toContain("66.7%");
  });

  it("shows an em dash instead of a fabricated percentage when iat is absent", () => {
    // No iat means the total lifetime is unknowable, so no honest percentage exists.
    // The countdown is still exact because it comes straight from exp.
    render(<AuthStatusBadge wallet={wallet} jwt={makeJwt({ exp: NOW_SEC + 3600 })} />);
    expect(pctLabel()).toContain("—");
    expect(pctLabel()).not.toContain("100.0%");
    expect(countdown()).toBe("1h 0m 0s");
  });

  it.each([
    ["iat equal to exp", { iat: NOW_SEC, exp: NOW_SEC }],
    ["iat after exp", { iat: NOW_SEC + 60, exp: NOW_SEC }],
    ["iat zero", { iat: 0, exp: NOW_SEC + 3600 }],
  ])("does not derive a bogus lifetime from %s", (_label, payload) => {
    render(<AuthStatusBadge wallet={wallet} jwt={makeJwt(payload)} />);
    // Either the token is already expired, or the percentage is withheld — in
    // neither case may it claim a lifetime derived from an impossible `iat`.
    const label = pctLabel();
    if (label.includes("%")) expect(label).toContain("—");
  });

  it("tolerates a malformed jwt without throwing", () => {
    expect(() => render(<AuthStatusBadge wallet={wallet} jwt="not-a-jwt" />)).not.toThrow();
    // Lifetime is unknowable, so the bar must not show a number.
    expect(pctLabel()).toContain("—");
  });
});

describe("expiry ticker (#1243)", () => {
  it("counts down from a single 1s timer", () => {
    render(<AuthStatusBadge wallet={wallet} jwt={makeJwt({ iat: NOW_SEC, exp: NOW_SEC + 3600 })} />);
    expect(countdown()).toBe("1h 0m 0s");

    act(() => {
      jest.advanceTimersByTime(1000);
    });
    expect(countdown()).toBe("0h 59m 59s");

    act(() => {
      jest.advanceTimersByTime(60_000);
    });
    // 3600 - 61 = 3539s left
    expect(countdown()).toBe("0h 58m 59s");
  });

  it("flips to EXPIRED once the clock reaches exp", () => {
    render(<AuthStatusBadge wallet={wallet} jwt={makeJwt({ iat: NOW_SEC, exp: NOW_SEC + 10 })} />);
    expect(screen.queryAllByText("EXPIRED")).toHaveLength(0);

    act(() => {
      jest.advanceTimersByTime(10_000);
    });
    // "EXPIRED" appears in both the status banner and the countdown label.
    expect(screen.getAllByText("EXPIRED").length).toBeGreaterThan(0);
    expect(countdown()).toBe("0h 0m 0s");
  });

  it("creates exactly one interval, at 1s, and no 30s twin", () => {
    // The previous implementation started a 30s interval and a 1s interval that
    // both recomputed the same values from the same state.
    const spy = jest.spyOn(global, "setInterval");
    render(<AuthStatusBadge wallet={wallet} jwt={makeJwt({ iat: NOW_SEC, exp: NOW_SEC + 3600 })} />);
    const tickers = spy.mock.calls.filter(([, delay]) => delay === 1000 || delay === 30000);
    expect(tickers).toHaveLength(1);
    expect(tickers[0][1]).toBe(1000);
    spy.mockRestore();
  });

  it("does not rebuild its timer when expiry state flips", () => {
    // The old effect depended on `isExpired`, so crossing the boundary tore down
    // and recreated both intervals.
    const spy = jest.spyOn(global, "setInterval");
    const { rerender } = render(
      <AuthStatusBadge wallet={wallet} jwt={makeJwt({ iat: NOW_SEC, exp: NOW_SEC + 5 })} />,
    );
    act(() => {
      jest.advanceTimersByTime(5000);
    });
    expect(screen.getAllByText("EXPIRED").length).toBeGreaterThan(0);
    rerender(
      <AuthStatusBadge wallet={wallet} jwt={makeJwt({ iat: NOW_SEC, exp: NOW_SEC + 5 })} />,
    );
    expect(spy).toHaveBeenCalledTimes(1);
    spy.mockRestore();
  });

  it("clears its interval on unmount", () => {
    const clearSpy = jest.spyOn(global, "clearInterval");
    const { unmount } = render(
      <AuthStatusBadge wallet={wallet} jwt={makeJwt({ iat: NOW_SEC, exp: NOW_SEC + 3600 })} />,
    );
    unmount();
    expect(clearSpy).toHaveBeenCalled();
    clearSpy.mockRestore();
  });
});
