/// <reference path="style_export.d.ts" />
/// <reference path="style_export2.d.ts" />
{
  const presentation = {};
  const scriptUrl = /** @type {HTMLScriptElement} */ (document.currentScript)
    .src;

  ///////////////////////////////////////////////////////////////////////////////
  // xx Utility, globals

  const notnull = /** @type {<T>(x: T | null | undefined) => T} */ (x) => {
    if (x == null) {
      throw Error();
    }
    return x;
  };

  const e = /** @type {
    <N extends keyof HTMLElementTagNameMap>(
      name: N,
      args: Partial<HTMLElementTagNameMap[N]>,
      args2: {
        styles_?: string[];
        children_?: Element[];
      }
    ) => HTMLElementTagNameMap[N]
  } */ (name, args1, args2) => {
    const out = document.createElement(name);
    if (args1 != null) {
      for (const [k, v] of Object.entries(args1)) {
        // @ts-ignore
        out[k] = v;
      }
    }
    if (args2.children_ != null) {
      for (const c of args2.children_) {
        out.appendChild(c);
      }
    }
    if (args2.styles_ != null) {
      for (const c of args2.styles_) {
        out.classList.add(c);
      }
    }
    return out;
  };

  const et = /** @type { 
      (
        markup: string,
        args?: {
          styles_?: string[];
          children_?: Element[];
        }
      ) => HTMLElement
    } */ (t, args) => {
    const out0 = document.createElement("div");
    out0.innerHTML = t;
    const out = /** @type {HTMLElement} */ (out0.firstElementChild);
    if (args != null) {
      if (args.styles_ != null) {
        for (const c of args.styles_) {
          out.classList.add(c);
        }
      }
      if (args.children_ != null) {
        for (const c of args.children_) {
          out.appendChild(c);
        }
      }
    }
    return out;
  };

  const globalStyle = new CSSStyleSheet();
  document.adoptedStyleSheets.push(globalStyle);
  globalStyle.insertRule(`:root {}`);
  const globalStyleRoot = /** @type { CSSStyleRule } */ (
    globalStyle.cssRules[globalStyle.cssRules.length - 1]
  ).style;
  for (const rule of [
    `@font-face { font-family: "I"; src: url("MaterialSymbolsRounded.woff2"); }`,
    `@font-face { font-family: "X"; src: url("KleeOne-Regular.ttf"); font-weight: 400; font-style: normal; }`,
    `@font-face { font-family: "X"; src: url("KleeOne-SemiBold.ttf"); font-weight: 600; font-style: normal; }`,
  ]) {
    globalStyle.insertRule(rule, globalStyle.cssRules.length);
  }
  const lightClause = "(prefers-color-scheme: light)";
  const globalStyleMediaLight =
    /** @type { CSSMediaRule } */
    (globalStyle.cssRules[globalStyle.insertRule(`@media ${lightClause} {}`)]);
  const globalStyleLight =
    /** @type { CSSStyleRule} */
    (
      globalStyleMediaLight.cssRules[
        globalStyleMediaLight.insertRule(":root {}")
      ]
    ).style;
  const globalStyleMediaDark =
    /** @type { CSSMediaRule } */
    (
      globalStyle.cssRules[
        globalStyle.insertRule(`@media not ${lightClause} {}`)
      ]
    );
  const globalStyleDark =
    /** @type { CSSStyleRule} */
    (globalStyleMediaDark.cssRules[globalStyleMediaDark.insertRule(":root {}")])
      .style;
  const wideClause = "(width >= 16cm)";
  const globalStyleMediaWide =
    /** @type { CSSMediaRule } */
    (globalStyle.cssRules[globalStyle.insertRule(`@media ${wideClause} {}`)]);
  const globalStyleMediaNarrow =
    /** @type { CSSMediaRule } */
    (
      globalStyle.cssRules[
        globalStyle.insertRule(`@media not ${wideClause} {}`)
      ]
    );

  const v = /** @type {(id: string, v: string) => string} */ (id, val) => {
    const name = `--${id}`;
    globalStyleRoot.setProperty(name, val);
    return `var(${name})`;
  };

  const vs = /** @type {(id:String, light: string, dark: string) => string} */ (
    id,
    light,
    dark,
  ) => {
    const name = `--${id}`;
    globalStyleLight.setProperty(name, light);
    globalStyleDark.setProperty(name, dark);
    return `var(${name})`;
  };

  const uniq = /** @type {(args: string|string[]) => string} */ (args) => {
    /** @type {string[]} */
    var args1;
    if (typeof args == "string") {
      args1 = [args];
    } else {
      args1 = args;
    }
    const lines = [];
    for (const e of notnull(new Error().stack).matchAll(/(\d+):\d+/g)) {
      lines.push(`${e[1]}`);
    }
    let uniq = [lines[1]];
    uniq.push(...args1.map((x) => x.replaceAll(/[^_a-zA-Z0-9]/g, "_")));
    return `r${uniq.join("_")}`;
  };
  /** @type { Set<string> } */
  const staticStyles = new Set();
  // Static style - the id must be unique for every value closed on (i.e. put all the arguments in the id).
  const s = /** @type {(
    id: string|string[],
    f: { [s: string]: (r: CSSStyleDeclaration) => void }
  ) => string} */ (id, f) => {
    let id1 = uniq(id);
    if (staticStyles.has(id1)) {
      return id1;
    }
    for (const [p, f1] of Object.entries(f)) {
      const suffix = p;
      globalStyle.insertRule(`.${id1}${suffix} {}`, 0);
      f1(/** @type { CSSStyleRule } */ (globalStyle.cssRules[0]).style);
    }
    staticStyles.add(id1);
    return id1;
  };
  const sm = /** @type {(
    id: string|string[],
    f: { [s: string]: { [m in "wide"|"narrow"|""]: (r: CSSStyleDeclaration) => void} }
  ) => string} */ (id, f) => {
    let id1 = uniq(id);
    if (staticStyles.has(id1)) {
      return id1;
    }
    for (const [suffix, f2] of Object.entries(f)) {
      for (const [m, f1] of Object.entries(f2)) {
        switch (m) {
          case "":
            globalStyle.insertRule(`.${id1}${suffix} {}`, 0);
            f1(/** @type { CSSStyleRule } */ (globalStyle.cssRules[0]).style);
            break;
          case "narrow":
            globalStyleMediaNarrow.insertRule(`.${id1}${suffix} {}`, 0);
            f1(
              /** @type { CSSStyleRule } */ (globalStyleMediaNarrow.cssRules[0])
                .style,
            );
            break;
          case "wide":
            globalStyleMediaWide.insertRule(`.${id1}${suffix} {}`, 0);
            f1(
              /** @type { CSSStyleRule } */ (globalStyleMediaWide.cssRules[0])
                .style,
            );
            break;
          default:
            throw new Error();
        }
      }
    }
    staticStyles.add(id1);
    return id1;
  };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Constants

  // xx Variables
  const varCBackground = vs("background", "whitesmoke", "rgb(70, 73, 77)");
  const varCBackgroundDark = vs(
    "background-dark",
    "rgba(219, 222, 238, 1)",
    "rgb(70, 73, 77)",
  );
  // TODO set BackgroundDark to match glass on bg (make glass slightly darker)
  const varCBackgroundGlass = vs(
    "background-dark",
    "rgba(219, 222, 238, 1)",
    "rgb(70, 73, 77)",
  );
  const varCForeground = vs(
    "c-foreground",
    "rgb(30, 30, 30)",
    "rgb(244, 255, 255)",
  );
  const varCQr = vs("c-qr", "black", "white");
  const varCForegroundChatButton = vs(
    "c-foreground-chat-button",
    "rgba(86, 103, 220, 1)",
    "rgb(244, 255, 255)",
  );
  const varCForegroundLight = `color-mix(in srgb, ${varCForeground} 80%, transparent)`;
  const varCForegroundVeryLight = `color-mix(in srgb, ${varCForeground} 50%, transparent)`;
  const varCForegroundUltraLight = `color-mix(in srgb, ${varCForeground} 20%, transparent)`;
  const varCForegroundHeadCenter = `color-mix(in srgb, ${varCForeground} 50%, ${varCBackground})`;
  const varCMutateForeground = vs(
    "c-foreground-mutate",
    "rgb(120, 79, 132)",
    "rgb(0,0,0)",
  );
  const varCNotifyForeground = "white";
  const varCNotifyBackground = vs(
    "c-foreground-notify",
    //"rgb(164, 32, 73)",
    "rgba(68, 78, 111, 1)",
    "rgb(0,0,0)",
  );
  const varCNotifyBright = vs(
    "c-foreground-notify-bright",
    //"rgb(164, 32, 73)",
    "rgba(81, 101, 176, 1)",
    "rgb(0,0,0)",
  );
  const varCForegroundError = vs(
    "c-foreground-error",
    "rgb(154, 60, 74)",
    "rgb(243, 69, 95)",
  );

  const varOIcon = "0.75";

  const varFMenu = "14pt";
  const varFCodeScale = "1.15em";
  const varFHeadBar = "13pt";

  const varSHeadButton = "0.7cm";
  const varSPageNarrow = "min(100%, 20cm)";
  const varSChatEntry = "16cm";
  const varSPortrait = "1.5cm";
  const varSChatControlsButton = "0.6cm";
  const varSMenuIcon = "1cm";
  const varSTopButtonIcon = "0.7cm";
  const varSFormButtonIcon = "0.47cm";
  const varSUnread = "0.3cm";

  const varPPage = "0.3cm";
  const varPSmall = "0.2cm";
  const varPIconButton = "0.07cm";
  const varPChatEntry = "0.2cm";
  const varPChatSpinner = "0.15cm";
  const varPBubblePadding = "0.18cm";

  const varWHead = "600";
  const varWIconMenuDecor = "300";
  const varWIconHead = "200";

  const varLAsyncSvg = "0.02";
  const varLThin = "0.06cm";
  const varLVeryThin = "0.05cm";

  const varRPortrait = "0.2cm";
  const varRBubble = "0.3cm";
  const varRInput = "0.2cm";
  const varRSelectable = "0.1cm";

  // xx State classes

  const classStateThinking = "thinking";
  presentation.classStateThinking =
    /** @type { Presentation["classStateThinking"]} */ () => ({
      value: classStateThinking,
    });
  const classStateSelected = "selected";
  presentation.classStateSelected =
    /** @type { Presentation["classStateSelected"]} */ () => ({
      value: classStateSelected,
    });
  const classStateHidden = "hidden";
  presentation.classStateHidden =
    /** @type { Presentation["classStateHidden"]} */ () => ({
      value: classStateHidden,
    });
  const classStateSticky = "sticky";
  presentation.classStateSticky =
    /** @type { Presentation["classStateSticky"]} */ () => ({
      value: classStateSticky,
    });
  const classStateControlsHidden = "controls_hidden";
  presentation.classStateControlsHidden =
    /** @type { Presentation["classStateControlsHidden"]} */ () => ({
      value: classStateControlsHidden,
    });
  const classStateRecording = "recording";
  presentation.classStateRecording =
    /** @type { Presentation["classStateRecording"]} */ () => ({
      value: classStateRecording,
    });
  const classStateVideo = "video";
  presentation.classStateVideo =
    /** @type { Presentation["classStateVideo"]} */ () => ({
      value: classStateVideo,
    });

  // xx Text classes

  const classCode = s("code", {
    "": (s) => {
      s.fontFamily = "monospace, monospace";
      s.fontSize = varFCodeScale;
      s.pointerEvents = "initial";
    },
  });
  presentation.classCode = /** @type { Presentation["classCode"]} */ () => ({
    value: classCode,
  });

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: all

  const contGroupStyle = "group";
  const contVboxStyle = "vbox";
  const contHboxStyle = "hbox";
  const contStackStyle = "stack";

  presentation.contGroup = /** @type {Presentation["contGroup"]} */ (args) => ({
    root: e("div", {}, { styles_: [contGroupStyle], children_: args.children }),
  });

  const menuItemLineHeight = "0.8cm";
  const paperStyle = s("paper", {
    "": (s) => {
      s.maskSize = "3cm, 3cm";
      s.maskRepeat = "repeat";
      s.maskPosition = "center";
      s.maskImage = `url("paper_mask.png")`;
      s.maskMode = "luminance";
      s.opacity = "0.8";
    },
  });

  document.addEventListener("DOMContentLoaded", () => {
    /*
CSS was designed by monkeys. In any other animation system, for 1. each property you'd define
2. frame times (segments) and then 3. interpolation functions between them.

In CSS, syntax/hierarchy wise, you'd expect to define 1. a an interpolation function an animation time, and then
modify 2. properties over 3. segments of the curve. This is less useful and more painful but okay, whatever.

The absurdly unintuitive truth, _counterindicated_ by the syntax: for 1. each property, 2. for each segment
referencing that property (i.e. if a frame doesn't include a property, skip it), 3. apply the single
interpolation function over that segment.

Why are the frames for different properties all mixed together? Why is only a single interpolation function
allowed? Why is the property the deepest element? How do you come up with something this crazy?
*/

    document.body.appendChild(
      et(`
      <style>
@keyframes spinner_opac {
  80% {
    opacity: 1;
  }
  82% {
    opacity: 0;
  }
  100% {
    opacity: 0;
  }
}

@keyframes spinner_dash {
  0% {
    stroke-dashoffset: 1;
  }
  30% {
    stroke-dashoffset: 0;
  }
  100% { 
    stroke-dashoffset: 0;
  }
}
  
svg.spinner1 path {
  animation: 
    spinner_dash 2s linear(
      0, 0.005, 0.021 6.4%, 0.029 7.6%, 0.054, 0.087 13.5%, 0.123 16.1%,
      0.176 19.6%, 0.356 30.3%, 0.437, 0.51, 0.568 46%, 0.633 52.9%, 0.701 61%,
      0.779 71.2%, 0.928 91.8%, 0.969 96.8%, 1
    ) infinite,
    spinner_opac 2s linear infinite
    ;
}

svg.spinner2 path {
  animation: 
    spinner_dash 2s linear(
      0, 0.004 3.3%, 0.015 6.2%, 0.033 8.8%, 0.058 11.2%, 0.084, 0.117 14.7%,
      0.258 20.7%, 0.284, 0.303 23.8%, 0.326 26.4%, 0.346 29.4%, 0.4 39.7%,
      0.421 43.3%, 0.445 46.6%, 0.465 49.1%, 0.505 53.2%, 0.551, 0.6 60.8%,
      0.736 70.1%, 0.776, 0.811 76.3%, 0.848 80.1%, 0.882, 0.912 88.2%, 0.966 96.2%,
      0.982 98.2%, 1
    ) infinite,
    spinner_opac 2s linear infinite
    ;
}

svg.spinner3 path {
  animation: 
    spinner_dash 2.3s linear(
      0, 0.003 2.5%, 0.01 4.7%, 0.022 6.6%, 0.038 8.3%, 0.055, 0.075 10.7%,
      0.154 14.2%, 0.185 15.8%, 0.211 17.7%, 0.234, 0.251 22.2%, 0.267 24.8%,
      0.314 34.7%, 0.342 39.6%, 0.374 44%, 0.41 47.9%, 0.439 50.4%, 0.463 52.1%,
      0.499 54.2%, 0.581 58.3%, 0.604 59.7%, 0.623 61.3%, 0.651 64.2%, 0.713 72.2%,
      0.774 79.8%, 0.853 88.5%, 0.895 92.7%, 0.939 96.5%, 0.978 99%, 1
    ) infinite,
    spinner_opac 2.3s linear infinite
    ;
}

svg.spinner4 path {
  animation: 
    spinner_dash 2.5s linear(
      0, 0.002, 0.009, 0.02 4.8%, 0.034 6%, 0.065 7.8%, 0.175 12.7%, 0.2, 0.22,
      0.234 17.4%, 0.246 19.4%, 0.28 27.5%, 0.291 29%, 0.303 30.2%, 0.341 32.5%,
      0.408 35.4%, 0.433 36.7%, 0.459 38.5%, 0.485, 0.503 42.9%, 0.518 45.3%,
      0.531 48.1%, 0.561 56%, 0.574 58.4%, 0.589 60.4%, 0.605 62%, 0.622 63.2%,
      0.641 64.2%, 0.688 66%, 0.712 67.1%, 0.738 68.8%, 0.764 71%, 0.785 73.9%,
      0.833 83.5%, 0.857 87.2%, 0.883, 0.915 93.1%, 1
    ) infinite,
    spinner_opac 2.5s linear infinite
    ;
}
      </style>
    `),
    );
  });
  const leafSpinner = /** @type { (extraStyles: string[])=>HTMLElement } */ (
    extraStyles,
  ) => {
    const styles = [
      s("spinner", {
        "": (s) => {
          //s.height = menuItemLineHeight;
          //s.width = "3cm";
        },
        " path": (s) => {
          s.strokeDasharray = "1";
          s.strokeDashoffset = "1";
        },
      }),
      paperStyle,
      ...extraStyles,
    ];
    const sel = Math.random();
    if (sel < 0.25) {
      return et(
        `
      <svg viewBox="0 0 1 0.35" xmlns="http://www.w3.org/2000/svg" class="${styles.join(
        " ",
      )} spinner1">
        <path fill="none" stroke="currentColor" stroke-width="${varLAsyncSvg}" pathLength="1" d="m 0.04276819,0.30609795 c 0.02501087,-0.0621192 0.02587843,-0.26140339 0.07010972,-0.25775973 0.0442308,0.0036438 0.0392081,0.24571051 0.0949588,0.24268559 0.0557507,-0.003024 0.044678,-0.19950842 0.10249788,-0.1955405 0.0578198,0.0039683 0.0658219,0.15775144 0.13449927,0.15586294 0.0686774,-0.001889 0.10746733,-0.0981084 0.17282507,-0.095961 0.0653576,0.002148 0.11054793,0.0614271 0.1828686,0.0649971 0.0723208,0.00357 0.0987762,-0.0464728 0.16302072,-0.0447597" />
      </svg>
    `,
      );
    } else if (sel < 0.5) {
      return et(
        `
      <svg viewBox="0 0 1 0.35" xmlns="http://www.w3.org/2000/svg" class="${styles.join(
        " ",
      )} spinner2">
        <path fill="none" stroke="currentColor" stroke-width="${varLAsyncSvg}" pathLength="1" d="M 0.14254063,0.2938125 C 0.10301621,0.2571751 0.0885351,0.17576883 0.12872349,0.11336434 c 0.0401883,-0.06240458 0.17293408,-0.08274137 0.25797572,-0.003932 0.0850417,0.0788098 0.0269571,0.13671136 -0.039893,0.13923062 -0.0668499,0.002519 -0.0812133,-0.0627161 -0.0300127,-0.0891682 0.0512009,-0.0264521 0.15399772,0.020863 0.23776893,0.0435695 0.0837712,0.0227066 0.19943644,0.028629 0.27049542,4.1993e-4 0.0710589,-0.0282097 0.0922414,-0.0677874 0.0456478,-0.0856104" />
      </svg>
    `,
      );
    } else if (sel < 0.75) {
      return et(
        `
      <svg viewBox="0 0 1 0.35" xmlns="http://www.w3.org/2000/svg" class="${styles.join(
        " ",
      )} spinner3">
        <path fill="none" stroke="currentColor" stroke-width="${varLAsyncSvg}" pathLength="1" d="M 0.05805971,0.13660498 C 0.10797019,0.26193567 0.35525482,0.19161688 0.38586466,0.11202727 0.41647444,0.03243767 0.3367885,0.0220047 0.31256276,0.08803155 0.28833702,0.1540584 0.27688391,0.22803629 0.38859489,0.24382805 0.50030587,0.2596199 0.66125404,0.11789295 0.71967761,0.22511469 0.77810135,0.3323364 0.59090405,0.3361921 0.60743783,0.21194022 0.62397152,0.08768833 0.8381139,0.13047976 0.94682009,0.15652352" />
      </svg>
    `,
      );
    } else {
      return et(
        `
      <svg viewBox="0 0 1 0.35" xmlns="http://www.w3.org/2000/svg" class="${styles.join(
        " ",
      )} spinner4">
        <path fill="none" stroke="currentColor" stroke-width="${varLAsyncSvg}" pathLength="1" d="m 0.06717016,0.19875525 c 0.04590072,0.0640954 0.09986502,0.0862412 0.14118426,0.0637398 0.0413194,-0.0225014 0.0786624,-0.0661734 0.0763519,-0.12579028 -0.002313,-0.05961708 -0.0802163,-0.07410729 -0.0724987,0.0114052 0.007718,0.0855125 0.0981388,0.10332275 0.14289414,0.10209764 0.0447553,-0.001225 0.14567377,-0.0514403 0.14452406,-0.11976192 -0.001152,-0.06832152 -0.0713155,-0.06170884 -0.077116,0.006882 -0.0058,0.0685904 0.079857,0.11707331 0.13892941,0.11242721 0.0590725,-0.004646 0.14949369,-0.0504459 0.14921933,-0.11892872 -2.7486e-4,-0.06848265 -0.0729905,-0.07196589 -0.0718564,0.006953 0.001134,0.0789188 0.0699061,0.11761026 0.12504166,0.11801568 0.0551356,4.0553e-4 0.1429375,-0.0578232 0.16859285,-0.10290166" />
      </svg>
    `,
      );
    }
  };

  presentation.leafAsyncBlock = /** @type {Presentation["leafAsyncBlock"]} */ (
    args,
  ) => {
    const inner = e(
      "div",
      {},
      {
        styles_: [
          contStackStyle,
          s(["leaf_async_block"], {
            "": (s) => {
              s.gridColumn = "1 / 3";
              s.justifyItems = "center";
              s.alignItems = "center";
            },
          }),
        ],
        children_: [
          leafSpinner([
            s("leaf_async_block_spinner", {
              "": (s) => {
                s.height = menuItemLineHeight;
              },
            }),
          ]),
        ],
      },
    );
    return {
      root: inner,
    };
  };

  presentation.leafErrBlock = /** @type {Presentation["leafErrBlock"]} */ (
    args,
  ) => {
    const out = e(
      "div",
      {},
      {
        styles_: [
          contStackStyle,
          s(["err_block"], {
            "": (s) => {
              s.flexGrow = "1";
              s.justifyItems = "center";
              s.alignItems = "center";
            },
            ">span": (s) => {},
          }),
        ],
        children_: [
          e(
            "span",
            { textContent: args.data },
            {
              styles_: [
                s("err_block_span", {
                  "": (s) => {
                    s.color = varCForegroundError;
                    s.pointerEvents = "initial";
                  },
                }),
              ],
            },
          ),
        ],
      },
    );
    return {
      root: out,
    };
  };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: root

  presentation.contRoot = /** @type { Presentation["contRoot"] } */ (args) => {
    const page = e(
      "div",
      {},
      {
        styles_: [
          sm("cont_root_wide_page", {
            "": {
              "": (s) => {
                s.position = "relative";
                s.display = "flex";
                s.flexDirection = "column";
                s.justifyContent = "stretch";
                s.height = "100dvh";
                s.overflowY = "scroll";
                s.pointerEvents = "initial";
                s.gridRow = "1";
              },
              wide: (s) => {
                s.gridColumn = "3";
              },
              narrow: (s) => {
                s.gridColumn = "1";
              },
            },
          }),
        ],
      },
    );
    return {
      page: page,
      root: e(
        "div",
        {},
        {
          styles_: [
            sm("cont_root_wide", {
              "": {
                "": (s) => {
                  s.display = "grid";
                },
                wide: (s) => {
                  s.gridTemplateColumns = "8cm 0fr auto";
                  //s.columnGap = "0.5cm";
                },
                narrow: (s) => {
                  s.gridTemplateColumns = "1fr";
                },
              },
            }),
          ],
          children_: [
            e(
              "div",
              {},
              {
                styles_: [
                  s("cont_root_wide_top", {
                    "": (s) => {
                      s.gridColumn = "1";
                      s.gridRow = "1";

                      s.position = "relative";

                      s.display = "flex";
                      s.flexDirection = "column";
                      s.justifyContent = "stretch";
                      s.height = "100dvh";
                      s.overflowY = "scroll";
                      s.pointerEvents = "initial";
                    },
                  }),
                ],
                children_: [args.menu],
              },
            ),
            /*
            e(
              "div",
              {},
              {
                styles_: [
                  sm("cont_root_wide_sep", {
                    "": {
                      "": (s) => {},
                      wide: (s) => {
                        s.gridColumn = "2";
                        s.justifySelf = "center";
                        s.alignSelf = "center";
                        s.height = "80%";
                        s.borderLeftWidth = "0.07cm";
                        s.borderLeftStyle = "dotted";
                        s.borderLeftColor = varCForegroundUltraLight;
                      },
                      narrow: (s) => {
                        s.display = "none";
                      },
                    },
                  }),
                ],
              },
            ),
            */
            page,
          ],
        },
      ),
    };
  };
  presentation.contPageBlank = /** @type { Presentation["contPageBlank"] } */ (
    args,
  ) => {
    return { root: e("div", {}, {}) };
  };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: menu, form, top

  const menuItemStyle0 = s("leaf_menu_item0", {
    "": (s) => {
      s.height = varSMenuIcon;
    },
  });
  const onwhiteSelectableRules =
    /** @type { { [s: string]: (r: CSSStyleDeclaration) => void } } */ ({
      "": (s) => {
        s.borderRadius = varRSelectable;
        s.border = `${varLThin} solid transparent`;
        s.pointerEvents = "initial";
      },
      [`:hover`]: (s) => {
        s.backgroundColor = varCBackgroundDark;
      },
      [`.${classStateSelected}:hover`]: (s) => {
        s.backgroundColor = `color-mix(in srgb, ${varCBackgroundDark} 50%, transparent)`;
        s.borderColor = varCBackgroundDark;
      },
      [`.${classStateSelected}`]: (s) => {
        s.backgroundColor = varCBackgroundDark;
      },
    });
  const onwhiteSelectableStyle = s("top_selectable", onwhiteSelectableRules);

  const unreadStyle = s("unread", {
    "": (s) => {
      s.backgroundColor = varCNotifyBright;
      s.backgroundBlendMode = "multiply";
      s.color = varCNotifyForeground;
      s.borderRadius = "999cm";
      s.width = varSUnread;
      s.height = varSUnread;
      s.margin = "0.2cm";
    },
    [`.${classStateHidden}`]: (s) => {
      s.display = "none";
    },
  });
  const backUnreadStyles = [
    unreadStyle,
    classStateHidden,
    sm("unread_back", {
      "": {
        narrow: (s) => {},
        wide: (s) => {
          s.display = "none";
        },
        "": (s) => {},
      },
    }),
  ];
  const menuTextStyle = s("leaf_menu_text", {
    "": (s) => {
      s.flexBasis = "0";
      s.flexGrow = "1";
      s.textOverflow = "ellipsis";
      s.overflowX = "hidden";
      s.whiteSpace = "nowrap";
      s.fontSize = varFMenu;
      s.fontWeight = "600";
      s.maxWidth = "100%";
    },
  });
  const leafMenuTextUnread =
    /** @type (args: {text: string})=> {all: HTMLElement[], text: HTMLElement, unread: HTMLElement} */ (
      args,
    ) => {
      const text = e(
        "span",
        { textContent: args.text },
        {
          styles_: [menuTextStyle],
        },
      );
      const unread = e("div", {}, { styles_: [unreadStyle, classStateHidden] });
      return {
        all: [text, unread],
        text: text,
        unread: unread,
      };
    };
  const portraitClipStyle = s("portrait_clip", {
    "": (s) => {
      s.borderRadius = varRPortrait;
    },
  });
  const menuImage =
    /** @type {(args: {image?: string, generatedImage?: string}) => {src: string, styles: string[]} | null} */ (
      args,
    ) => {
      if (args.image != null) {
        return { src: args.image, styles: [portraitClipStyle] };
      }
      if (args.generatedImage != null) {
        return { src: args.generatedImage, styles: [] };
      }
      return null;
    };
  const menuTextUnreadOuterStyles = [
    contHboxStyle,
    s("leaf_menu_text_unread_outer", {
      "": (s) => {
        s.gridColumn = "2";
        s.justifyContent = "space-between";
        s.alignItems = "center";
        s.minWidth = "0";
        s.paddingLeft = "0.1cm";
      },
    }),
  ];
  presentation.leafMenuLinkImage =
    /** @type { Presentation["leafMenuLinkImage"] } */ (args) => {
      const image = menuImage(args);
      if (image == null) {
        throw new Error("leafMenuLinkImage requires image or generatedImage");
      }
      return {
        root: e(
          "img",
          { src: image.src },
          {
            styles_: [
              ...image.styles,
              s("leaf_menu_link_icon", {
                "": (s) => {
                  s.minWidth = "0";
                  s.minHeight = "0";
                  s.maxWidth = "100%";
                  s.maxHeight = "100%";
                },
              }),
            ],
          },
        ),
      };
    };
  presentation.leafMenuLink = /** @type { Presentation["leafMenuLink"] } */ (
    args,
  ) => {
    /** @type { HTMLElement[] } */
    const children = [];
    /** @type { HTMLElement | undefined } */
    let imageOuter = undefined;
    if (menuImage(args) != null) {
      const outerPadding = "0.1cm";
      imageOuter = e(
        "div",
        {},
        {
          styles_: [
            s("leaf_menu_link_icon_outer", {
              "": (s) => {
                s.gridColumn = "1";
                s.display = "grid";
                s.alignItems = "center";
                s.justifyItems = "center";
                s.padding = outerPadding;
              },
            }),
          ],
          children_: [presentation.leafMenuLinkImage(args).root],
        },
      );
      children.push(imageOuter);
    }
    const textUnread = leafMenuTextUnread({ text: args.text });
    children.push(
      e(
        "div",
        {},
        {
          styles_: [
            ...menuTextUnreadOuterStyles,
            menuItemStyle0,
            onwhiteSelectableStyle,
          ],
          children_: textUnread.all,
        },
      ),
    );
    return {
      root: e(
        "a",
        { href: args.link },
        {
          styles_: [
            menuItemStyle0,
            s("leaf_menu_link", {
              "": (s) => {
                s.display = "grid";
                s.gridTemplateColumns = "subgrid";
                s.gridColumn = "1 / 3";
                s.alignItems = "center";
                s.maxWidth = "100%";
              },
              [`.${classStateSelected}>.${onwhiteSelectableStyle}`]: (s) => {
                s.backgroundColor = varCBackgroundDark;
              },
              [`.${classStateSelected}>.${onwhiteSelectableStyle}:hover`]: (s) => {
                s.backgroundColor = `color-mix(in srgb, ${varCBackgroundDark} 50%, transparent)`;
                s.borderColor = varCBackgroundDark;
              },
            }),
          ],
          children_: children,
        },
      ),
      image: imageOuter,
      text: textUnread.text,
      unread: textUnread.unread,
    };
  };

  presentation.leafMenuButton =
    /** @type { Presentation["leafMenuButton"] } */ (args) => {
      return {
        root: e(
          "button",
          { textContent: args.text },
          {
            styles_: [
              onwhiteSelectableStyle,
              ...menuTextUnreadOuterStyles,
              menuTextStyle,
              menuItemStyle0,
            ],
          },
        ),
      };
    };

  presentation.leafMenuGroup = /** @type { Presentation["leafMenuGroup"] } */ (
    args,
  ) => {
    const groupEl = e(
      "div",
      {},
      {
        styles_: [
          ...menuGridStyles,
          s("leaf_menu_group_group", {
            "": (s) => {
              s.marginLeft = "0.7cm";
            },
          }),
        ],
        children_: args.children,
      },
    );
    const toggleStyle = s("leaf_menu_group_toggle", {
      "": (s) => {
        s.gridColumn = "1";
        s.pointerEvents = "initial";
        s.cursor = "pointer";
        s.userSelect = "none";
        s.fontFamily = "I";
        s.fontWeight = varWIconMenuDecor;
        s.flexGrow = "0";
        s.alignItems = "center";
        s.height = s.width = varSMenuIcon;
        s.display = "grid";
        s.gridTemplateColumns = "1fr";
        s.justifyItems = "center";
        s.alignItems = "center";
        s.opacity = varOIcon;
      },
    });
    const textUnread = leafMenuTextUnread({ text: args.text });
    /** @type { HTMLElement[] } */
    const linkChildren = [];
    const image = menuImage(args);
    if (image != null) {
      linkChildren.push(
        e(
          "img",
          { src: image.src },
          {
            styles_: [
              ...image.styles,
              s("leaf_menu_group_icon", {
                "": (s) => {
                  s.width = s.height = `calc(${varSMenuIcon} - 0.2cm)`;
                  s.marginRight = "0.2cm";
                  s.flexShrink = "0";
                },
              }),
            ],
          },
        ),
      );
    }
    linkChildren.push(...textUnread.all);
    const link = e(
      "a",
      { href: args.link },
      {
        styles_: [
          ...menuTextUnreadOuterStyles,
          menuItemStyle0,
          onwhiteSelectableStyle,
        ],
        children_: linkChildren,
      },
    );
    return {
      root: e(
        "details",
        {},
        {
          styles_: [
            s("leaf_menu_group_details", {
              "": (s) => {
                s.gridColumn = "1 / 3";
              },
              [`[open]>summary>.${toggleStyle}`]: (s) => {
                s.rotate = "90deg";
              },
              ">summary::marker": (s) => {
                s.display = "none";
                s.content = '""';
              },
            }),
          ],
          children_: [
            e(
              "summary",
              {},
              {
                styles_: [menuGridRowStyle],
                children_: [
                  e(
                    "div",
                    {},
                    {
                      styles_: [
                        toggleStyle,
                        menuItemStyle0,
                        onwhiteSelectableStyle,
                      ],
                      children_: [
                        leafSvg({
                          width: "0.7cm",
                          text: svgIconClosed,
                        }),
                      ],
                    },
                  ),
                  link,
                ],
              },
            ),
            groupEl,
          ],
        },
      ),
      group: groupEl,
      link: link,
      text: textUnread.text,
      unread: textUnread.unread,
    };
  };

  presentation.leafMenuCode = /** @type { Presentation["leafMenuCode"] } */ (
    args,
  ) => {
    const copy = e(
      "button",
      { title: "Copy" },
      {
        styles_: [
          onwhiteSelectableStyle,
          s("leaf_menu_code_copy", {
            "": (s) => {
              s.display = "grid";
              s.justifyItems = "center";
              s.alignItems = "center";
              s.padding = varPIconButton;
            },
          }),
        ],
        children_: [
          leafSvg({
            text: svgIconCopy,
            height: varSFormButtonIcon,
            extraStyles: [
              s("leaf_menu_code_copy_icon", {
                "": (s) => {
                  s.opacity = varOIcon;
                },
              }),
            ],
          }),
        ],
      },
    );
    return {
      root: e(
        "div",
        {},
        {
          styles_: [
            contHboxStyle,
            s("leaf_menu_code", {
              "": (s) => {
                s.gridColumn = "1 / 3";
                s.alignItems = "center";
                s.gap = varPSmall;
                s.minWidth = "0";
              },
            }),
          ],
          children_: [
            e(
              "code",
              { textContent: args.text },
              {
                styles_: [
                  classCode,
                  s("leaf_menu_code_text", {
                    "": (s) => {
                      s.flexGrow = "1";
                      s.flexShrink = "1";
                      s.minWidth = "0";
                      s.overflow = "hidden";
                      s.textOverflow = "ellipsis";
                      s.whiteSpace = "nowrap";
                      s.backgroundColor = varCBackgroundDark;
                      s.borderRadius = varRSelectable;
                      s.padding = "0.1cm 0.2cm";
                      s.userSelect = "all";
                    },
                  }),
                ],
              },
            ),
            copy,
          ],
        },
      ),
      copy: copy,
    };
  };

  presentation.leafMenuQr = /** @type { Presentation["leafMenuQr"] } */ (
    args,
  ) => {
    const image = e(
      "div",
      {},
      {
        styles_: [
          s("leaf_menu_qr_image", {
            "": (s) => {
              s.width = "min(10cm, 100%)";
              s.aspectRatio = "1";
              s.backgroundColor = varCQr;
              s.maskSize = "100% 100%";
            },
          }),
        ],
      },
    );
    image.style.maskImage = `url(${JSON.stringify(`data:image/svg+xml,${encodeURIComponent(args.svg)}`)})`;
    return {
      root: e(
        "div",
        {},
        {
          styles_: [
            s("leaf_menu_qr", {
              "": (s) => {
                s.gridColumn = "1 / 3";
                s.display = "grid";
                s.justifyItems = "center";
                s.padding = "0.1cm 0";
              },
            }),
          ],
          children_: [image],
        },
      ),
    };
  };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: menu, form
  const onBlueButtonColors = s("onblue_button_colors", {
    "": (s) => {
      s.borderRadius = "9999cm";
    },
    [`.${classStateSelected}`]: (s) => {
      s.backgroundColor = varCBackground;
    },
    ":hover": (s) => {
      s.backgroundColor = varCBackground;
    },
    [`.${classStateSelected}:hover`]: (s) => {
      s.backgroundColor = `color-mix(in srgb, ${varCBackground} 50%, transparent)`;
    },
  });
  const onBlueButtonInverseColors = s("onblue_button_colors_inverse", {
    "": (s) => {
      s.borderRadius = "9999cm";
    },
    [`.${classStateSelected}`]: (s) => {
      s.backgroundColor = varCBackgroundDark;
    },
    ":hover": (s) => {
      s.backgroundColor = varCBackgroundDark;
    },
    [`.${classStateSelected}:hover`]: (s) => {
      s.backgroundColor = `color-mix(in srgb, ${varCBackgroundDark} 50%, transparent)`;
    },
  });
  const onBlueIconButtonStyle = s("onblue_icon_button", {
    "": (s) => {
      s.display = "grid";
      s.gridTemplateColumns = "1fr";
      s.justifyItems = "center";
      s.alignItems = "center";
      s.width = s.height = "0.8cm";
    },
  });
  const onBlueIconButtonStyles = [onBlueButtonColors, onBlueIconButtonStyle];
  const onBlueIconButtonInverseStyles = [
    onBlueButtonInverseColors,
    onBlueIconButtonStyle,
  ];
  const headIconButton =
    /** @type { (_:{link: string, svg: string, extraStyles?: string[]}) => HTMLElement } */ (
      args,
    ) => {
      return e(
        "a",
        { href: args.link },
        {
          styles_: [...onBlueIconButtonStyles, ...(args.extraStyles || [])],
          children_: [
            leafSvg({
              text: args.svg,
              width: varSHeadButton,
              extraStyles: [
                s("head_icon_button", {
                  "": (s) => {
                    s.color = varCForegroundVeryLight;
                  },
                }),
              ],
            }),
          ],
        },
      );
    };

  presentation.contNonchatHeadBar =
    /** @type { Presentation["contNonchatHeadBar"] } */ (args) => {
      const backUnread = e("div", {}, { styles_: backUnreadStyles });
      const back = headIconButton({
        svg: svgIconBack,
        link: args.backLink,
      });
      const children = [];
      children.push(
        e(
          "div",
          {},
          {
            styles_: [
              s("cont_head_bar_left", {
                "": (s) => {
                  s.gridColumn = "1";

                  s.display = "flex";
                  s.flexDirection = "row";
                  s.justifyContent = "start";
                  s.alignItems = "center";
                },
              }),
            ],
            children_: [back, backUnread],
          },
        ),
      );
      children.push(
        e(
          "div",
          {},
          {
            styles_: [
              s("cont_head_bar_center", {
                "": (s) => {
                  s.gridColumn = "2";

                  s.display = "flex";
                  s.flexDirection = "row";
                  s.justifyContent = "center";
                  s.alignItems = "center";
                },
              }),
            ],
            children_: [args.center],
          },
        ),
      );
      if (args.right != null) {
        children.push(
          e(
            "div",
            {},
            {
              styles_: [
                s("cont_head_bar_right", {
                  "": (s) => {
                    s.gridColumn = "3";

                    s.display = "flex";
                    s.flexDirection = "row";
                    s.justifyContent = "end";
                    s.alignItems = "center";
                  },
                }),
              ],
              children_: [args.right],
            },
          ),
        );
      }
      return {
        root: floatingBar({
          extraStyles: [
            s("cont_head_bar", {
              "": (s) => {
                s.marginTop = "0.2cm";
                s.marginBottom = "0.3cm";
                s.display = "grid";
                s.gridTemplateColumns = "1fr auto 1fr";
              },
            }),
          ],
          children_: children,
        }),
        back: back,
        backUnread: backUnread,
      };
    };

  presentation.leafMenuQrScanner =
    /** @type { Presentation["leafMenuQrScanner"] } */ (args) => {
      const video = e(
        "video",
        { muted: true, playsInline: true, autoplay: true },
        {
          styles_: [
            s("leaf_menu_qr_scanner_video", {
              "": (s) => {
                s.width = "min(10cm, 100%)";
                s.aspectRatio = "1";
                s.objectFit = "cover";
                s.borderRadius = varRBubble;
                s.backgroundColor = varCQr;
              },
            }),
          ],
        },
      );
      const status = e("div", {}, { styles_: [errorStyle] });
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              s("leaf_menu_qr_scanner", {
                "": (s) => {
                  s.gridColumn = "1 / 3";
                  s.display = "grid";
                  s.justifyItems = "center";
                  s.rowGap = varPSmall;
                  s.padding = "0.1cm 0";
                },
              }),
            ],
            children_: [video, status],
          },
        ),
        video: video,
        status: status,
      };
    };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: top

  presentation.contPageTop = /** @type { Presentation["contPageTop"] } */ (
    args,
  ) => {
    const appiconSize = "1.1cm";
    const topButton = /** @type { (svg: string, link: string)=>HTMLElement} */ (
      svg,
      link,
    ) => {
      return e(
        "a",
        { href: link },
        {
          styles_: [
            onwhiteSelectableStyle,
            s("cont_page_top_button", {
              "": (s) => {
                s.display = "grid";
                s.gridTemplateColumns = "1fr";
                s.justifyItems = "center";
                s.alignItems = "center";
                s.padding = "0.1cm";
                s.margin = "0.1cm";
              },
            }),
          ],
          children_: [
            leafSvg({
              text: svg,
              height: varSTopButtonIcon,
              extraStyles: [
                s("cont_page_top_generic_button", {
                  "": (s) => {
                    s.opacity = varOIcon;
                  },
                }),
              ],
            }),
          ],
        },
      );
    };
    const settingsLink = topButton(svgIconSettings, args.settingsLink);
    const identitiesLink = topButton(svgIconIdentity, args.identitiesLink);
    const addLink = topButton(svgIconAdd, args.addLink);
    return {
      root: e(
        "div",
        {},
        {
          styles_: [contVboxStyle],
          children_: [
            e(
              "div",
              {},
              {
                styles_: [
                  contHboxStyle,
                  s("cont_page_top_hbox_outer", {
                    "": (s) => {
                      s.padding = "1cm 1cm 0.7cm 1cm";
                      s.gap = "0.13cm";
                      s.alignItems = "center";
                      s.justifyContent = "center";
                    },
                  }),
                ],
                children_: [
                  e(
                    "img",
                    { src: "inapp_icon.svg" },
                    {
                      styles_: [
                        paperStyle,
                        s("cont_page_top_appicon", {
                          "": (s) => {
                            s.marginRight = "0.15cm";
                            s.width = appiconSize;
                            s.height = appiconSize;
                          },
                        }),
                      ],
                    },
                  ),
                  settingsLink,
                  identitiesLink,
                  addLink,
                ],
              },
            ),
            e(
              "div",
              {},
              {
                styles_: [
                  ...menuGridStyles,
                  s("cont_page_top", {
                    "": (s) => {
                      s.margin = `0 0.6cm`;
                    },
                  }),
                ],
                children_: args.body,
              },
            ),
          ],
        },
      ),
      addLink: addLink,
      settingsLink: settingsLink,
      identitiesLink: identitiesLink,
    };
  };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: menu

  const menuGridRowStyle = s("menu_grid_row", {
    "": (s) => {
      s.display = "grid";
      s.gridTemplateColumns = `${varSMenuIcon} auto`;
      s.justifyItems = "stretch";
      s.alignItems = "center";
      s.columnGap = "0.1cm";
    },
  });
  const menuGridStyles = [
    menuGridRowStyle,
    s("menu_grid", {
      "": (s) => {
        s.rowGap = "0.1cm";
        s.padding = "0.1cm 0";
      },
    }),
  ];
  const nonchatPageStyleInner = s("nonchat_cont_page", {
    "": (s) => {
      s.margin = varPPage;
      s.position = "relative";
    },
  });
  const bubbleStyle = s("bubble", {
    "": (s) => {
      s.borderRadius = varRBubble;
      s.padding = varPBubblePadding;
    },
  });
  const nonchatPageStylesOuter = [
    s("cont_page_outer", {
      "": (s) => {
        s.flexGrow = "1";
      },
    }),
  ];
  presentation.contPageMenu = /** @type { Presentation["contPageMenu"] } */ (
    args,
  ) => {
    return {
      root: e(
        "div",
        {},
        {
          styles_: [contVboxStyle, ...nonchatPageStylesOuter],
          children_: [
            args.headBar,
            e(
              "div",
              {},
              {
                styles_: [
                  nonchatPageStyleInner,
                  ...menuGridStyles,
                  s("cont_page_menu_inner", {
                    "": (s) => {
                      s.width = varSPageNarrow;
                      s.alignSelf = "center";
                    },
                  }),
                ],
                children_: args.children,
              },
            ),
          ],
        },
      ),
    };
  };

  const removeSuffix = /** @type { (suff: string, all: string) => string } */ (
    suff,
    all,
  ) => {
    if (!all.endsWith(suff)) {
      throw new Error();
    }
    return all.substring(0, all.length - suff.length);
  };
  const panic = /** @type { <T>() => T } */ () => {
    throw new Error();
  };
  const leafSvg =
    /** @type {(args: {width?: string, height?: string, text: string, extraStyles?: string[]})=>HTMLElement} */ (
      args,
    ) => {
      const width =
        args.width ?? args.height ?? /** @type { string } */ (panic());
      const height =
        args.height ?? args.width ?? /** @type { string } */ (panic());
      const widthFloat = Number.parseFloat(removeSuffix("cm", width));
      const heightFloat = Number.parseFloat(removeSuffix("cm", height));
      var thicknessFloat = Number.parseFloat(removeSuffix("cm", varLVeryThin));
      var relWidth;
      var relHeight;
      var relThickness;
      if (args.width != null) {
        relWidth = 1;
        relHeight = heightFloat / widthFloat;
        relThickness = thicknessFloat / widthFloat;
      } else {
        relHeight = 1;
        relWidth = widthFloat / heightFloat;
        relThickness = thicknessFloat / heightFloat;
      }
      relWidth *= 10;
      relHeight *= 10;
      relThickness *= 10;
      return et(
        `
        <svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${relWidth} ${relHeight}" fill="none" stroke="currentColor" stroke-width="${relThickness}">
        ${args.text}
        </svg>
      `,
        { styles_: args.extraStyles || [] },
      );
    };
  // All 10x10 to work with leafSvg
  const svgIconSettings = `<path d="M 4.31771,0.90964 5.73455,0.91664 6.09313,2.18873 6.97669,2.80134 8.28887,2.48698 8.91655,3.55729 7.91238,4.48391 7.92458,5.63636 8.94791,6.50111 8.36387,7.59541 7.21054,7.33344 6.09733,7.92532 5.82051,9.1754 4.25358,9.1919 3.93448,7.89527 2.87611,7.33578 1.68562,7.65025 1.01037,6.59993 1.95733,5.59742 1.96433,4.4247 1.0661,3.48549 1.68666,2.46798 2.92087,2.80557 3.87497,2.22525 Z" />
      <circle cx="5.0333862" cy="5.0034509" r="1.4575578" /> `;
  const svgIconIdentity = `<ellipse cx="5.0000134" cy="2.7737503" rx="1.7285849" ry="1.7067041" />
            <path d="m 1.70009,9.0854 c 0,0 -0.0757,-1.9327733 0.43401,-2.3333933 0.5097,-0.40061 1.85863,-0.71435 2.81093,-0.76661 0.95229,-0.0522 2.58176,0.41876 2.94221,0.73492 0.36044,0.31616 0.41667,2.3779933 0.41667,2.3779933" />
`;
  const svgIconAdd = `<path d="m 1.22036,4.91196 7.27082,0.0512" />
      <path d="M 4.90698,1.22536 5.00934,8.77702" /> `;
  const svgIconNope = `<path d="M 1.78044,1.95892 7.96763,7.90814" />
      <path d="M 7.84864,1.95892 1.95892,8.08661" /> `;
  const svgIconBack = `<path d="m 6.3569633,1.06654 -3.86699,3.98597 3.92649,3.98598" />`;
  const svgIconSend = `<path d="M 1.94858,2.01502 8.95692,5.16878 2.00698,8.03051" />
      <path d="M 0.78052,4.93516 8.7233,5.16878" /> `;
  const svgIconBall = `<circle cx="5.0333862" cy="5.0154572" r="1.4575578" /> `;
  const svgIconOpened = `<path d="M 2.38412,3.35249 4.97757,6.69406 7.52115,3.30262" /> `;
  const svgIconUp = `<path d="M 2.38412,6.64751 4.97757,3.30594 7.52115,6.69738" /> `;
  const svgIconCopy = `<rect x="3.5" y="1.5" width="5" height="5.5" rx="0.8" />
      <path d="M 3.5,3 H 2.3 Q 1.5,3 1.5,3.8 V 7.7 Q 1.5,8.5 2.3,8.5 H 5.7 Q 6.5,8.5 6.5,7.7 V 7" /> `;
  const svgIconEdit = `<path d="M 2,8 2.4,6.2 6.8,1.8 8.2,3.2 3.8,7.6 Z" />
      <path d="M 5.9,2.7 7.3,4.1" /> `;
  const svgIconHistory = `<circle cx="5" cy="5" r="3.6" />
      <path d="M 5,2.9 V 5 L 6.5,6" /> `;
  const svgIconCalendar = `<rect x="1.5" y="2.5" width="7" height="6" rx="0.8" />
      <path d="M 1.5,4.5 H 8.5" />
      <path d="M 3.5,1.5 V 3.2" />
      <path d="M 6.5,1.5 V 3.2" /> `;
  const svgIconCheck = `<path d="M 1.8,5.2 4,7.4 8.4,2.6" /> `;
  const svgIconTrash = `<path d="M 2,3 H 8" />
      <path d="M 4,3 V 1.8 H 6 V 3" />
      <path d="M 2.8,3 3.3,8.5 H 6.7 L 7.2,3" /> `;
  const svgIconCamera = `<rect x="1" y="3" width="8" height="5.5" rx="0.8" />
      <path d="M 3.5,3 4.2,1.8 H 5.8 L 6.5,3" />
      <circle cx="5" cy="5.7" r="1.6" /> `;
  const svgIconVideo = `<rect x="1" y="2.8" width="5.6" height="4.4" rx="0.6" />
      <path d="M 6.6,4.4 9,3 V 7 L 6.6,5.6" /> `;
  const svgIconFile = `<path d="M 2.5,1 H 6 L 7.5,2.5 V 9 H 2.5 Z" />
      <path d="M 6,1 V 2.5 H 7.5" /> `;
  const svgIconText = `<path d="M 2,2 H 8" />
      <path d="M 5,2 V 8.5" /> `;
  const svgIconFlip = `<path d="M 2,4 A 3.2,3.2 0 0 1 8,4" />
      <path d="M 8,1.8 V 4 H 5.8" />
      <path d="M 8,6 A 3.2,3.2 0 0 1 2,6" />
      <path d="M 2,8.2 V 6 H 4.2" /> `;
  const svgIconDownload = `<path d="M 5,1.5 V 6.5" />
      <path d="M 2.8,4.5 5,6.7 7.2,4.5" />
      <path d="M 1.5,8.5 H 8.5" /> `;
  const svgIconPlay = `<path d="M 3,2 8,5 3,8 Z" /> `;
  const svgIconForward = `<path d="m 3.6430367,1.06654 3.86699,3.98597 -3.92649,3.98598" />`;
  const svgIconClosed = `<path d="M 3.31381,2.55686 6.54797,4.97396 3.31381,7.42511" /> `;
  const svgIconMessage = `<path        d="M 1.0451019,9.0882899 C 1.881773,6.8346388 0.9807132,6.4106016 1.0025419,4.7517916 1.0270541,2.8890523 2.5540914,1.2417577 5.1503393,1.2344073 7.7465873,1.2270557 9.178426,2.801595 9.1403442,4.7019845 9.1022618,6.6023739 7.7306769,8.016585 5.0856639,8.0388243 3.4647788,8.0524527 2.7929907,7.4356862 1.0451019,9.0882899 Z"
/>`;

  const svgMaskImage = /** @type {(text: string, size: string) => string} */ (
    text,
    size,
  ) => {
    const sizeFloat = Number.parseFloat(removeSuffix("cm", size));
    const thicknessFloat = Number.parseFloat(removeSuffix("cm", varLVeryThin));
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10" fill="none" stroke="black" stroke-width="${(thicknessFloat / sizeFloat) * 10}">${text}</svg>`;
    return `url(${JSON.stringify(`data:image/svg+xml,${encodeURIComponent(svg)}`)})`;
  };

  presentation.leafNonchatHeadBarCenterPlaceholder =
    /** @type { Presentation["leafNonchatHeadBarCenterPlaceholder"] } */ (
      args,
    ) => {
      return { root: e("span", { textContent: "..." }, {}) };
    };

  presentation.leafNonchatHeadBarCenter =
    /** @type { Presentation["leafNonchatHeadBarCenter"] } */ (args) => {
      if (args.link == null) {
        return {
          root: e(
            "span",
            { textContent: args.text },
            { styles_: [leafHeadCenterStyle] },
          ),
        };
      } else {
        return {
          root: e(
            "a",
            { textContent: args.text, href: args.link },
            {
              styles_: leafHeadCenterButtonStyles,
            },
          ),
        };
      }
    };

  presentation.leafMenuHeadBarRightAdd =
    /** @type { Presentation["leafMenuHeadBarRightAdd"] } */ (args) => {
      return { root: headIconButton({ link: args.link, svg: svgIconAdd }) };
    };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: form

  const spacer = () => {
    return e(
      "div",
      {},
      {
        styles_: [
          s("spacer", {
            "": (s) => {
              s.flexGrow = "1";
            },
          }),
        ],
      },
    );
  };
  const errorStyle = s("error", {
    "": (s) => {
      s.color = varCForegroundError;
    },
  });
  const formGridStyle = /** @type { (s: CSSStyleDeclaration) => void } */ (
    s,
  ) => {
    s.display = "grid";
    s.gridTemplateColumns = "minmax(0, max-content) minmax(60%, 1fr)";
    s.columnGap = varPSmall;
    s.rowGap = varPSmall;
    s.alignItems = "center";
  };
  presentation.contPageForm = /** @type { Presentation["contPageForm"] } */ (
    args,
  ) => {
    const errors = e("div", {}, { styles_: [errorStyle] });
    const submit = e(
      "button",
      { textContent: "Ok" },
      {
        styles_: [
          s("button_mut_text_outline", {
            "": (s) => {
              s.border = `${varLThin} solid transparent`;
              s.fontFamily = "sans-serif";
              s.fontSize = "14pt";
              s.fontWeight = "500";
              s.padding = "0.2cm 0.4cm";
              s.fontFamily = "sans-serif";
              s.color = varCMutateForeground;
              s.borderRadius = "0.2cm";
            },
            ":hover": (s) => {
              s.borderColor = varCMutateForeground;
            },
          }),
        ],
      },
    );
    const body = e(
      "div",
      {},
      {
        styles_: [
          ...menuGridStyles,
          nonchatPageStyleInner,
          s("cont_page_form_inner", {
            "": (s) => {
              s.position = "relative";
              s.width = varSPageNarrow;
              s.alignSelf = "center";
            },
            ...Object.fromEntries(
              Object.entries(onwhiteSelectableRules).map(([suffix, f]) => [
                ` .rf_button_icon${suffix}`,
                f,
              ]),
            ),
            ">*": (s) => {
              s.gridColumn = "2";
            },
            ">form": formGridStyle,
            " :where(form, .rf_subform, .group) > *": (s) => {
              s.gridColumn = "2";
            },
            " .rf_label": (s) => {
              s.gridColumn = "1";
            },
            " .rf_vec_item_header": (s) => {
              s.gridColumn = "1 / -1";
            },
            " .rf_vec": (s) => {
              s.gridColumn = "1 / -1";
            },
            " .rf_file": (s) => {
              s.display = "flex";
              s.flexWrap = "wrap";
              s.alignItems = "center";
              s.gap = varPSmall;
            },
            " .rf_file>input": (s) => {
              s.flexGrow = "1";
            },
            " .rf_file_preview": (s) => {
              s.flexBasis = "100%";
              s.display = "grid";
              s.justifyItems = "center";
            },
            " .rf_file_preview:empty": (s) => {
              s.display = "none";
            },
            " .rf_file_preview>*": (s) => {
              s.width = "min(10cm, 100%)";
            },
            " .rf_button_icon::before": (s) => {
              s.content = JSON.stringify("");
              s.display = "block";
              s.width = varSFormButtonIcon;
              s.height = varSFormButtonIcon;
              s.margin = varPIconButton;
              s.backgroundColor = "currentColor";
              s.maskSize = "100% 100%";
              s.opacity = varOIcon;
            },
            " .rf_button_add::before": (s) => {
              s.maskImage = svgMaskImage(svgIconAdd, varSFormButtonIcon);
            },
            " .rf_button_delete::before": (s) => {
              s.maskImage = svgMaskImage(svgIconNope, varSFormButtonIcon);
            },
            " .rf_button_move_up::before": (s) => {
              s.maskImage = svgMaskImage(svgIconUp, varSFormButtonIcon);
            },
            " .rf_button_move_down::before": (s) => {
              s.maskImage = svgMaskImage(svgIconOpened, varSFormButtonIcon);
            },
            " .rf_subform": (s) => {
              formGridStyle(s);
              s.gridColumn = "1 / -1";
              s.borderLeft = `${varLThin} solid ${varCBackgroundGlass}`;
              s.paddingLeft = varPSmall;
            },
            " .rf_input_big": (s) => {
              s.gridColumn = "1 / -1";
            },
            " :is(input, select, [contenteditable])": (s) => {
              s.border = `${varLThin} solid ${varCBackgroundGlass}`;
              s.borderRadius = varRInput;
              s.padding = varPSmall;
            },
            " input[type=checkbox]": (s) => {
              s.appearance = "none";
              s.minWidth = varSMenuIcon;
              s.minHeight = varSMenuIcon;
              s.padding = "0";
              s.verticalAlign = "middle";
              s.cursor = "pointer";
            },
            " input[type=checkbox]:hover": (s) => {
              s.backgroundColor = varCBackgroundDark;
            },
            " input[type=checkbox]:checked": (s) => {
              s.backgroundColor = varCBackgroundDark;
            },
            " input[type=checkbox]:checked:hover": (s) => {
              s.backgroundColor = `color-mix(in srgb, ${varCBackgroundDark} 50%, transparent)`;
            },
            " div.rf_input_small:has(> label > input[type=checkbox])": (s) => {
              s.display = "flex";
              s.flexWrap = "wrap";
              s.gap = varPSmall;
            },
            " label:has(> input[type=checkbox])": (s) => {
              s.display = "inline-grid";
              s.justifyItems = "center";
              s.alignItems = "center";
              s.minWidth = varSMenuIcon;
              s.minHeight = varSMenuIcon;
              s.borderRadius = varRInput;
              s.cursor = "pointer";
              s.pointerEvents = "initial";
            },
            " label:has(> input[type=checkbox]):hover": (s) => {
              s.backgroundColor = varCBackgroundDark;
            },
            " label:has(> input[type=checkbox]:checked)": (s) => {
              s.backgroundColor = varCBackgroundDark;
            },
            " label:has(> input[type=checkbox]:checked):hover": (s) => {
              s.backgroundColor = `color-mix(in srgb, ${varCBackgroundDark} 50%, transparent)`;
            },
            " label > input[type=checkbox]": (s) => {
              s.position = "absolute";
              s.opacity = "0";
              s.width = "0";
              s.height = "0";
              s.minWidth = "0";
              s.minHeight = "0";
              s.border = "none";
            },
            " .rf_error": (s) => {
              s.display = "block";
              s.gridColumn = "1 / -1";
              s.color = varCForegroundError;
            },
            " .rf_error:empty": (s) => {
              s.display = "none";
            },
            " .disable_hide": (s) => {
              s.display = "none";
            },
            " .rf_subform.disable_hide": (s) => {
              s.display = "none";
            },
            " .rf_error.disable_hide": (s) => {
              s.display = "none";
            },
          }),
        ],
        children_: [...args.children],
      },
    );
    return {
      root: e(
        "div",
        {},
        {
          styles_: [contVboxStyle, ...nonchatPageStylesOuter],
          children_: [
            args.headBar,
            body,
            spacer(),
            e(
              "div",
              {},
              {
                styles_: [
                  contVboxStyle,
                  s("cont_page_form_edit_bar", {
                    "": (s) => {
                      s.paddingLeft = "0.3cm";
                      s.paddingRight = "0.3cm";
                      s.alignItems = "end";
                      s.position = "sticky";
                      s.left = "0";
                      s.right = "0";
                      s.bottom = "0.3cm";
                    },
                  }),
                ],
                children_: [errors, submit],
              },
            ),
          ],
        },
      ),
      submit: submit,
      errors: errors,
      body: body,
    };
  };

  presentation.leafFormText = /** @type { Presentation["leafFormText"] } */ (
    args,
  ) => {
    return { root: e("div", { textContent: args.text }, {}) };
  };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: chat

  const classPageChat = s("cont_page_chat", {
    "": (s) => {
      s.backgroundColor = varCBackground;
      s.flexGrow = "1";
      s.position = "relative";
      s.pointerEvents = "initial";
    },
  });
  presentation.classPageChat =
    /** @type { Presentation["classPageChat"]} */ () => ({
      value: classPageChat,
    });

  const leafChatSpinner = () => {
    return e(
      "div",
      {},
      {
        styles_: [
          contHboxStyle,
          s("leaf_chat_spinner_outer", {
            "": (s) => {
              s.justifyContent = "center";
              s.padding = varPChatEntry;
            },
          }),
        ],
        children_: [
          e(
            "div",
            {},
            {
              styles_: [
                s("leaf_chat_spinner", {
                  "": (s) => {
                    s.width = "min-content";
                    s.display = "flex";
                    s.justifyContent = "center";
                    s.alignItems = "center";
                    s.padding = varPChatSpinner;
                  },
                }),
              ],
              children_: [
                leafSpinner([
                  s("chat_spinner", {
                    "": (s) => {
                      s.height = "0.8cm";
                    },
                  }),
                ]),
              ],
            },
          ),
        ],
      },
    );
  };
  presentation.leafChatSpinnerCenter =
    /** @type { Presentation["leafChatSpinnerCenter"] } */ (args) => {
      return {
        root: leafChatSpinner(),
      };
    };

  presentation.leafChatSpinnerEarly =
    /** @type { Presentation["leafChatSpinnerCenter"] } */ (args) => {
      return {
        root: leafChatSpinner(),
      };
    };

  presentation.leafChatSpinnerLate =
    /** @type { Presentation["leafChatSpinnerCenter"] } */ (args) => {
      return {
        root: leafChatSpinner(),
      };
    };

  const glassBlurStyle = s("glass_blur", {
    "": (s) => {
      s.display = "block";
      s.position = "absolute";
      s.inset = "0";
      s.zIndex = "-2";
      s.backdropFilter = `blur(0.2cm)`;
    },
  });
  const glassTintStyle = s("glass_tint", {
    "": (s) => {
      s.display = "block";
      s.position = "absolute";
      s.inset = "0";
      s.zIndex = "-1";
      s.opacity = "0.7";
    },
  });
  const glassLayers = /** @type (tintStyle: string) => HTMLElement[] */ (
    tintStyle,
  ) => {
    return [
      e("div", {}, { styles_: [glassBlurStyle] }),
      e("div", {}, { styles_: [glassTintStyle, tintStyle] }),
    ];
  };

  const floatingBar =
    /** @type (args: {extraStyles?: string[], children_: HTMLElement[]}) => HTMLElement */ (
      args,
    ) => {
      return e(
        "div",
        {},
        {
          styles_: [
            s("floating_bar", {
              "": (s) => {
                s.margin = `0.2cm`;
                s.padding = `0.1cm`;
                s.borderRadius = "0.4cm";
                s.position = "relative";
                s.isolation = "isolate";
                s.overflow = "hidden";
              },
            }),
            ...(args.extraStyles || []),
          ],
          children_: [
            ...glassLayers(
              s("floating_bar_tint", {
                "": (s) => {
                  s.background = varCBackgroundGlass;
                },
              }),
            ),
            ...args.children_,
          ],
        },
      );
    };

  presentation.contChatHeadBar =
    /** @type { Presentation["contChatHeadBar"] } */ (args) => {
      const children = [];
      const backUnread = e("div", {}, { styles_: backUnreadStyles });
      children.push(
        e(
          "div",
          {},
          {
            styles_: [
              s("cont_chat_bar_left", {
                "": (s) => {
                  s.gridColumn = "1";

                  s.display = "flex";
                  s.flexDirection = "row";
                  s.justifyContent = "start";
                  s.alignItems = "center";
                },
              }),
            ],
            children_: [
              headIconButton({
                link: args.backLink,
                svg: svgIconBack,
              }),
              backUnread,
            ],
          },
        ),
      );
      children.push(
        e(
          "div",
          {},
          {
            styles_: [
              s("cont_chat_bar_center", {
                "": (s) => {
                  s.gridColumn = "2";

                  s.display = "flex";
                  s.flexDirection = "row";
                  s.justifyContent = "center";
                  s.alignItems = "center";
                },
              }),
            ],
            children_: [args.center],
          },
        ),
      );
      if (args.right != null) {
        children.push(
          e(
            "div",
            {},
            {
              styles_: [
                s("cont_chat_bar_right", {
                  "": (s) => {
                    s.gridColumn = "3";

                    s.display = "flex";
                    s.flexDirection = "row";
                    s.justifyContent = "end";
                    s.alignItems = "center";
                  },
                }),
              ],
              children_: [args.right],
            },
          ),
        );
      }
      return {
        root: floatingBar({
          extraStyles: [
            s("cont_chat_bar", {
              "": (s) => {
                s.display = "grid";
                s.gridTemplateColumns = "1fr auto 1fr";
              },
            }),
          ],
          children_: children,
        }),
        backUnread: backUnread,
      };
    };

  const leafHeadCenterStyle = s("leaf_head_center", {
    "": (s) => {
      s.fontWeight = varWHead;
      s.fontSize = varFHeadBar;
      s.color = varCForegroundLight;
    },
  });
  const leafHeadCenterButtonStyles = [
    onBlueButtonColors,
    leafHeadCenterStyle,
    s("leaf_chat_center_button", {
      "": (s) => {
        s.borderRadius = "999cm";
        s.padding = `0 0.3cm`;
        s.display = "flex";
        s.flexDirection = "row";
        s.alignItems = "center";
      },
      ":before": (s) => {
        s.content = JSON.stringify("");
        s.borderRadius = "999cm";
        s.border = `${varLThin} solid ${varCForegroundVeryLight}`;
        s.display = "block";
        const size = "0.3cm";
        s.width = size;
        s.height = size;
        s.marginRight = varPSmall;
      },
    }),
  ];
  presentation.leafChatHeadBarCenterPlaceholder =
    /** @type { Presentation["leafChatHeadBarCenterPlaceholder"] } */ (
      args,
    ) => {
      return {
        root: e(
          "span",
          { textContent: "..." },
          { styles_: leafHeadCenterButtonStyles },
        ),
      };
    };

  presentation.leafChatHeadBarCenter =
    /** @type { Presentation["leafChatHeadBarCenter"] } */ (args) => {
      return {
        root: e(
          "a",
          { textContent: args.text, href: args.link },
          {
            styles_: leafHeadCenterButtonStyles,
          },
        ),
      };
    };

  // Entry
  const chatEntryActionsSpecificStyle = "chat_entry_actions_specific";
  const chatEntryGlassSpecificStyle = "chat_entry_glass_specific";
  const chatEntryBodySpecificStyle = "chat_entry_body_specific";
  const chatEntrySelectableStyle = s("chat_entry_selectable", {
    "": (s) => {
      s.pointerEvents = "initial";
      s.cursor = "pointer";
    },
    [` .${chatEntryActionsSpecificStyle}`]: (s) => {
      s.display = "none";
    },
    [`.${classStateSelected} .${chatEntryActionsSpecificStyle}`]: (s) => {
      s.display = "flex";
    },
    [` .${chatEntryGlassSpecificStyle}`]: (s) => {
      s.display = "none";
    },
    [`.${classStateSelected} .${chatEntryGlassSpecificStyle}`]: (s) => {
      s.display = "block";
    },
  });
  presentation.contChatEntry =
    /** @type { Presentation["contChatEntry"] } */ (args) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              s("chat_entry", {
                [`.${classStateSticky} > *`]: (s) => {
                  s.maxHeight = "100cqh";
                },
                [`.${classStateSticky} .${chatEntryBodySpecificStyle}`]: (s) => {
                  s.flexShrink = "1";
                  s.minHeight = "0";
                  s.overflowY = "auto";
                  s.overscrollBehavior = "contain";
                },
              }),
            ],
          },
        ),
      };
    };
  presentation.contChatEntryModeMessage =
    /** @type { Presentation["contChatEntryModeMessage"] } */ (args) => {
      const body = e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            bubbleStyle,
            chatEntryBodySpecificStyle,
            s("chat_entry_body", {
              "": (s) => {
                s.gap = varPSmall;
              },
            }),
          ],
        },
      );
      const actions = e(
        "div",
        {},
        {
          styles_: [
            chatEntryActionsSpecificStyle,
            s("chat_entry_actions", {
              "": (s) => {
                s.flexDirection = "row";
                s.gap = "0.1cm";
              },
            }),
          ],
        },
      );

      // The vertical content bit
      const outer3 = e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            s("chat_entry_mode_message_outer3", {
              "": (s) => {
                s.gap = "0.1cm";
                s.flexBasis = "0";
                s.flexGrow = "1";
              },
            }),
          ],
          children_: [
            e(
              "time",
              {
                nodeValue: args.date,
                textContent: new Date(args.date).toLocaleTimeString(),
              },
              {
                styles_: [
                  s("chat_stamp", {
                    "": (s) => {
                      s.fontSize = "9pt";
                      s.fontWeight = "600";
                    },
                  }),
                ],
              },
            ),
            body,
            actions,
          ],
        },
      );
      if (args.left) {
        outer3.style.alignItems = "start";
      } else {
        outer3.style.alignItems = "end";
      }

      const portrait = e(
        "div",
        {},
        {
          styles_: [
            s("chat_portrait", {
              "": (s) => {
                s.display = "grid";
                s.flexShrink = "0";
                s.width = varSPortrait;
                s.height = varSPortrait;
              },
            }),
          ],
        },
      );

      const glassPad = "0.5cm";
      const glassBorderGap = "0.1cm";
      const glassBorderRadius = "0.3cm";
      const glass = e(
        "div",
        {},
        {
          styles_: [
            chatEntryGlassSpecificStyle,
            s("chat_entry_glass", {
              "": (s) => {
                s.position = "absolute";
                s.inset = `calc(-1 * ${glassPad})`;
                s.zIndex = "-1";
                s.borderRadius = `calc(${glassBorderRadius} + ${glassPad} - ${glassBorderGap})`;
                s.isolation = "isolate";
                s.overflow = "hidden";
              },
            }),
          ],
          children_: [
            ...glassLayers(
              s("chat_entry_glass_tint", {
                "": (s) => {
                  s.background = varCBackground;
                },
              }),
            ),
            e(
              "div",
              {},
              {
                styles_: [
                  s("chat_entry_glass_border", {
                    "": (s) => {
                      s.position = "absolute";
                      s.inset = `calc(${glassPad} - ${glassBorderGap})`;
                      s.borderRadius = glassBorderRadius;
                      s.border = `${varLThin} solid ${varCNotifyBright}`;
                    },
                  }),
                ],
              },
            ),
          ],
        },
      );

      // The smallest box containing the actual content
      const outer2 = e(
        "div",
        {},
        {
          styles_: [
            contHboxStyle,
            s("chat_entry_mode_message_outer2", {
              "": (s) => {
                s.width = varSChatEntry;
                s.gap = varPSmall;
                s.flexShrink = "1";
                s.position = "relative";
                s.isolation = "isolate";
              },
            }),
          ],
          children_: [
            glass,
            portrait,
            outer3,
          ],
        },
      );
      if (args.left) {
        outer2.style.flexDirection = "row";
      } else {
        outer2.style.flexDirection = "row-reverse";
      }

      // A narrower box that contains enough space for left/right
      const outer1 = e(
        "div",
        {},
        {
          styles_: [
            contHboxStyle,
            s("chat_entry_mode_message_outer1", {
              "": (s) => {
                s.width = varSPageNarrow;
              },
            }),
          ],
          children_: [outer2],
        },
      );
      if (args.left) {
        outer1.style.justifyContent = "start";
      } else {
        outer1.style.justifyContent = "end";
      }

      // A box centering the content
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              contHboxStyle,
              chatEntrySelectableStyle,
              s("chat_entry_mode_message_outer", {
                "": (s) => {
                  s.justifyContent = "center";
                  s.padding = varPChatEntry;
                },
              }),
            ],
            children_: [outer1],
          },
        ),
        portrait: portrait,
        body: body,
        actions: actions,
      };
    };
  const chatEntryMessageButtonStyles = [
    ...onBlueIconButtonInverseStyles,
    s("chat_entry_message_button", {
      "": (s) => {
        s.color = varCForegroundChatButton;
      },
    }),
  ];
  presentation.leafChatEntryMessageButtonEdit =
    /** @type { Presentation["leafChatEntryMessageButtonEdit"] } */ (args) => {
      return {
        root: e(
          "button",
          { title: "Edit" },
          {
            styles_: chatEntryMessageButtonStyles,
            children_: [
              leafSvg({ text: svgIconEdit, width: varSChatControlsButton }),
            ],
          },
        ),
      };
    };
  presentation.leafChatEntryMessageButtonHistory =
    /** @type { Presentation["leafChatEntryMessageButtonHistory"] } */ (
      args,
    ) => {
      return {
        root: e(
          "a",
          { title: "History", href: args.link },
          {
            styles_: chatEntryMessageButtonStyles,
            children_: [
              leafSvg({ text: svgIconHistory, width: varSChatControlsButton }),
            ],
          },
        ),
      };
    };
  presentation.leafChatEntryPortrait =
    /** @type { Presentation["leafChatEntryPortrait"] } */ (args) => {
      const image = menuImage(args);
      if (image == null) {
        throw new Error("leafChatEntryPortrait requires image or generatedImage");
      }
      return {
        root: e(
          "img",
          { src: image.src },
          {
            styles_: [
              ...image.styles,
              s("chat_portrait_image", {
                "": (s) => {
                  s.width = "100%";
                  s.height = "100%";
                },
              }),
            ],
          },
        ),
      };
    };
  const mediaSquareStyle = s("media_square", {
    "": (s) => {
      s.position = "relative";
      s.display = "grid";
      s.gridTemplateColumns = "1fr";
      s.gridTemplateRows = "1fr";
      s.overflow = "hidden";
      s.borderRadius = varRSelectable;
      s.pointerEvents = "initial";
      s.backgroundColor = varCForegroundUltraLight;
    },
    ":hover": (s) => {
      s.opacity = varOIcon;
    },
  });
  presentation.leafMediaThumb = /** @type { Presentation["leafMediaThumb"] } */ (
    args,
  ) => {
    const mediaStyle = s("leaf_media_thumb_media", {
      "": (s) => {
        s.gridColumn = "1";
        s.gridRow = "1";
        s.width = "100%";
        s.height = "100%";
        s.objectFit = "cover";
      },
    });
    /** @type { HTMLElement[] } */
    const children = [];
    if (args.srcIsVideo) {
      children.push(
        e(
          "video",
          { src: args.src, preload: "metadata", muted: true, playsInline: true },
          { styles_: [mediaStyle] },
        ),
      );
    } else {
      children.push(
        e(
          "img",
          { src: args.src, loading: "lazy", decoding: "async" },
          { styles_: [mediaStyle] },
        ),
      );
    }
    if (args.video) {
      children.push(
        e(
          "div",
          {},
          {
            styles_: [
              s("leaf_media_thumb_play", {
                "": (s) => {
                  s.gridColumn = "1";
                  s.gridRow = "1";
                  s.placeSelf = "center";
                  s.display = "grid";
                  s.justifyItems = "center";
                  s.alignItems = "center";
                  s.width = s.height = "0.8cm";
                  s.borderRadius = "999cm";
                  s.backgroundColor = `color-mix(in srgb, black 50%, transparent)`;
                  s.color = "white";
                },
              }),
            ],
            children_: [leafSvg({ text: svgIconPlay, width: "0.5cm" })],
          },
        ),
      );
    }
    return {
      root: e(
        "div",
        {},
        {
          styles_: [
            s("leaf_media_thumb", {
              "": (s) => {
                s.display = "grid";
                s.gridTemplateColumns = "1fr";
                s.gridTemplateRows = "1fr";
                s.gridColumn = "1";
                s.gridRow = "1";
                s.width = "100%";
                s.height = "100%";
              },
            }),
          ],
          children_: children,
        },
      ),
    };
  };
  presentation.leafChatEntryMessageText =
    /** @type { Presentation["leafChatEntryMessageText"] } */ (args) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              s("chat_entry_text", {
                "": (s) => {
                  s.pointerEvents = "initial";
                },
              }),
            ],
            children_: [args.content],
          },
        ),
      };
    };
  const varSEntryMedia = "3cm";
  presentation.contChatEntryMessageMedia =
    /** @type { Presentation["contChatEntryMessageMedia"] } */ (args) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              s("cont_chat_entry_message_media", {
                "": (s) => {
                  s.display = "flex";
                  s.flexDirection = "row";
                  s.flexWrap = "wrap";
                  s.justifyContent = "center";
                  s.gap = "0.1cm";
                },
              }),
            ],
            children_: args.children,
          },
        ),
      };
    };
  presentation.leafChatEntryMessageMediaSquare =
    /** @type { Presentation["leafChatEntryMessageMediaSquare"] } */ (args) => {
      return {
        root: e(
          "button",
          {},
          {
            styles_: [
              mediaSquareStyle,
              s("leaf_chat_entry_message_media_square", {
                "": (s) => {
                  s.width = s.height = varSEntryMedia;
                },
              }),
            ],
            children_: [args.content],
          },
        ),
      };
    };
  presentation.leafChatEntryMessageExtension =
    /** @type { Presentation["leafChatEntryMessageExtension"] } */ (args) => {
      return {
        root: e(
          "details",
          {},
          {
            styles_: [
              s("leaf_chat_entry_message_extension", {
                "": (s) => {
                  s.pointerEvents = "initial";
                  s.minWidth = "0";
                },
                " > summary": (s) => {
                  s.cursor = "pointer";
                  s.color = varCForegroundLight;
                },
                " > summary:hover": (s) => {
                  s.color = varCForeground;
                },
              }),
            ],
            children_: [
              e(
                "summary",
                {},
                { children_: [e("code", { textContent: args.key }, { styles_: [classCode] })] },
              ),
              e(
                "pre",
                { textContent: args.json },
                {
                  styles_: [
                    classCode,
                    s("leaf_chat_entry_message_extension_json", {
                      "": (s) => {
                        s.pointerEvents = "initial";
                        s.whiteSpace = "pre";
                        s.overflowX = "auto";
                        s.marginTop = "0.1cm";
                        s.padding = "0.1cm 0.2cm";
                        s.borderRadius = varRSelectable;
                        s.backgroundColor = varCForegroundUltraLight;
                      },
                    }),
                  ],
                },
              ),
            ],
          },
        ),
      };
    };
  presentation.leafChatEntryMessageFile =
    /** @type { Presentation["leafChatEntryMessageFile"] } */ (args) => {
      return {
        root: e(
          "a",
          { href: args.link, download: args.name, title: args.name },
          {
            styles_: [
              contHboxStyle,
              s("leaf_chat_entry_message_file", {
                "": (s) => {
                  s.gap = "0.2cm";
                  s.alignItems = "center";
                  s.minWidth = "0";
                },
                ":hover": (s) => {
                  s.textDecoration = "underline";
                },
              }),
            ],
            children_: [
              leafSvg({
                text: svgIconFile,
                width: varSChatControlsButton,
                extraStyles: [
                  s("leaf_chat_entry_message_file_icon", {
                    "": (s) => {
                      s.color = varCForegroundVeryLight;
                    },
                  }),
                ],
              }),
              e(
                "span",
                { textContent: args.name },
                {
                  styles_: [
                    s("leaf_chat_entry_message_file_name", {
                      "": (s) => {
                        s.overflow = "hidden";
                        s.textOverflow = "ellipsis";
                        s.whiteSpace = "nowrap";
                        s.minWidth = "0";
                      },
                    }),
                  ],
                },
              ),
              e(
                "span",
                { textContent: args.size },
                {
                  styles_: [
                    s("leaf_chat_entry_message_file_size", {
                      "": (s) => {
                        s.color = varCForegroundLight;
                        s.fontSize = "9pt";
                      },
                    }),
                  ],
                },
              ),
            ],
          },
        ),
      };
    };

  presentation.contChatEntryModeDeleted =
    /** @type { Presentation["contChatEntryModeDeleted"] } */ (args) => {
      return { root: e("div", {}, {}) };
    };

  presentation.contChatEntryModeControls =
    /** @type { Presentation["contChatEntryModeControls"] } */ (args) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              contHboxStyle,
              s("leaf_chat_entry_mode_controls_outer", {
                "": (s) => {
                  s.justifyContent = "center";
                  s.gap = "0.2cm";
                  s.margin = `0.5cm 0`;
                },
              }),
            ],
          },
        ),
      };
    };

  presentation.leafChatEntryModeControlsButtonNewMessage =
    /** @type { Presentation["leafChatEntryModeControlsButtonNewMessage"] } */ (
      args,
    ) => {
      return {
        root: e(
          "button",
          {},
          {
            styles_: [
              ...onBlueIconButtonInverseStyles,
              s("leaf_chat_entry_mode_controls_button_new_message", {
                "": (s) => {
                  s.color = varCForegroundChatButton;
                },
              }),
            ],
            children_: [
              leafSvg({ text: svgIconMessage, width: varSChatControlsButton }),
            ],
          },
        ),
      };
    };

  // Controls
  const chatControlsButtonRowStyles = [
    contHboxStyle,
    s("leaf_chat_controls_mode_menu_buttons", {
      "": (s) => {
        s.gap = "0.2cm";
        s.justifyContent = "center";
      },
    }),
  ];
  const chatControlsIconButton = /** @type { (svg: string, title: string) => HTMLElement } */ (
    svg,
    title,
  ) => {
    return e(
      "button",
      { title: title },
      {
        styles_: [...onBlueIconButtonStyles],
        children_: [leafSvg({ text: svg, width: varSChatControlsButton })],
      },
    );
  };

  presentation.contChatControlsBarModeMenu =
    /** @type { Presentation["contChatControlsBarModeMenu"] } */ (args) => {
      return {
        root: floatingBar({
          children_: [
            e(
              "div",
              {},
              {
                styles_: chatControlsButtonRowStyles,
                children_: args.children,
              },
            ),
          ],
          extraStyles: [
            contHboxStyle,
            s("leaf_chat_controls_mode_menu", {
              "": (s) => {
                s.justifyContent = "center";
              },
            }),
          ],
        }),
      };
    };

  presentation.leafChatControlsBarModeMenuButtonNewMessage =
    /** @type { Presentation["leafChatControlsBarModeMenuButtonNewMessage"] } */ (
      args,
    ) => {
      return { root: chatControlsIconButton(svgIconMessage, "New message") };
    };

  const chatEditorLeftWidth = "0.8cm";
  const chatEditorTextStyle = s("leaf_chat_editor_text", {
    "": (s) => {
      s.display = "block";
      s.backgroundColor = `color-mix(in srgb, ${varCBackground} 50%, transparent)`;
      s.borderRadius = varRBubble;
      s.padding = "0.1cm 0.2cm";
      s.minWidth = "0";
    },
    " > .malarkdowney": (s) => {
      s.display = "block";
      s.minHeight = "1.2em";
      s.cursor = "text";
    },
  });
  const chatEditorBar =
    /** @type { (args: { sendSvg: string }) => { root: HTMLElement, rows: HTMLElement, send: HTMLElement } } */ (
      args,
    ) => {
      const send = e(
        "button",
        {},
        {
          styles_: [
            ...onBlueIconButtonStyles,
            s("chat_editor_message_send_button", {
              "": (s) => {
                s.alignSelf = "flex-end";
              },
            }),
          ],
          children_: [
            leafSvg({
              text: args.sendSvg,
              width: varSChatControlsButton,
              extraStyles: [
                s("chat_editor_message_send", {
                  "": (s) => {
                    s.color = varCMutateForeground;
                  },
                }),
              ],
            }),
          ],
        },
      );
      const rows = e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            s("chat_editor_message_rows", {
              "": (s) => {
                s.gap = "0.1cm";
                s.flexGrow = "1";
                s.flexBasis = "0";
                s.minWidth = "0";
              },
            }),
          ],
        },
      );
      return {
        root: floatingBar({
          children_: [rows, send],
          extraStyles: [
            contHboxStyle,
            s("leaf_chat_editor_mode_message", {
              "": (s) => {
                s.gap = "0.1cm";
              },
            }),
          ],
        }),
        rows: rows,
        send: send,
      };
    };

  presentation.leafChatEditorModeMessage =
    /** @type { Presentation["leafChatEditorModeMessage"] } */ (args) => {
      return chatEditorBar({ sendSvg: svgIconSend });
    };

  presentation.leafChatEditorModeEdit =
    /** @type { Presentation["leafChatEditorModeEdit"] } */ (args) => {
      return chatEditorBar({ sendSvg: svgIconEdit });
    };

  presentation.contChatEditorRow =
    /** @type { Presentation["contChatEditorRow"] } */ (args) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              contHboxStyle,
              s("cont_chat_editor_row", {
                "": (s) => {
                  s.gap = "0.1cm";
                  s.alignItems = "flex-start";
                },
              }),
            ],
            children_: [
              e(
                "div",
                {},
                {
                  styles_: [
                    contVboxStyle,
                    s("cont_chat_editor_row_left", {
                      "": (s) => {
                        s.width = chatEditorLeftWidth;
                        s.alignItems = "center";
                      },
                    }),
                  ],
                  children_: args.left,
                },
              ),
              e(
                "div",
                {},
                {
                  styles_: [
                    s("cont_chat_editor_row_middle", {
                      "": (s) => {
                        s.flexGrow = "1";
                        s.flexBasis = "0";
                        s.minWidth = "0";
                      },
                    }),
                  ],
                  children_: [args.middle],
                },
              ),
            ],
          },
        ),
      };
    };

  presentation.leafChatEditorButtonClose =
    /** @type { Presentation["leafChatEditorButtonClose"] } */ (args) => {
      return { root: chatControlsIconButton(svgIconNope, "Close") };
    };

  presentation.leafChatEditorButtonDelete =
    /** @type { Presentation["leafChatEditorButtonDelete"] } */ (args) => {
      return { root: chatControlsIconButton(svgIconTrash, "Delete") };
    };

  presentation.leafChatEditorButtonPlus =
    /** @type { Presentation["leafChatEditorButtonPlus"] } */ (args) => {
      return {
        root: e(
          "button",
          { title: "Add" },
          {
            styles_: [
              onBlueButtonColors,
              s("leaf_chat_editor_button_plus", {
                "": (s) => {
                  s.float = "right";
                  s.display = "grid";
                  s.justifyItems = "center";
                  s.alignItems = "center";
                  s.width = s.height = "0.55cm";
                  s.marginLeft = "0.1cm";
                  s.color = varCForegroundLight;
                },
              }),
            ],
            children_: [leafSvg({ text: svgIconAdd, width: "0.4cm" })],
          },
        ),
      };
    };

  presentation.contChatEditorToolbox =
    /** @type { Presentation["contChatEditorToolbox"] } */ (args) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: chatControlsButtonRowStyles,
            children_: args.children,
          },
        ),
      };
    };

  presentation.leafChatEditorToolboxButtonText =
    /** @type { Presentation["leafChatEditorToolboxButtonText"] } */ (args) => {
      return { root: chatControlsIconButton(svgIconText, "Add text") };
    };

  presentation.leafChatEditorToolboxButtonCamera =
    /** @type { Presentation["leafChatEditorToolboxButtonCamera"] } */ (
      args,
    ) => {
      return { root: chatControlsIconButton(svgIconCamera, "Camera") };
    };

  presentation.leafChatEditorToolboxButtonDelete =
    /** @type { Presentation["leafChatEditorToolboxButtonDelete"] } */ (
      args,
    ) => {
      return {
        root: e(
          "button",
          { title: "Delete message" },
          {
            styles_: [
              onBlueIconButtonStyle,
              s("leaf_chat_editor_toolbox_button_delete", {
                "": (s) => {
                  s.borderRadius = "9999cm";
                },
                ":hover": (s) => {
                  s.backgroundColor = varCForegroundError;
                  s.color = "white";
                },
                [`.${classStateSelected}`]: (s) => {
                  s.backgroundColor = varCForegroundError;
                  s.color = "white";
                },
                [`.${classStateSelected}:hover`]: (s) => {
                  s.backgroundColor = `color-mix(in srgb, ${varCForegroundError} 50%, transparent)`;
                },
              }),
            ],
            children_: [leafSvg({ text: svgIconTrash, width: varSChatControlsButton })],
          },
        ),
      };
    };

  presentation.leafChatEditorToolboxButtonFile =
    /** @type { Presentation["leafChatEditorToolboxButtonFile"] } */ (args) => {
      return { root: chatControlsIconButton(svgIconFile, "Add files") };
    };

  presentation.leafChatEditorText =
    /** @type { Presentation["leafChatEditorText"] } */ (args) => {
      const plusSlot = e("div", {}, { styles_: [contGroupStyle] });
      return {
        root: e(
          "div",
          {},
          { styles_: [chatEditorTextStyle], children_: [plusSlot, args.editor] },
        ),
        plusSlot: plusSlot,
      };
    };

  const varSControlsMedia = "2cm";
  presentation.contChatEditorMedia =
    /** @type { Presentation["contChatEditorMedia"] } */ (args) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              s("cont_chat_editor_media", {
                "": (s) => {
                  s.display = "grid";
                  s.gridTemplateColumns = `repeat(auto-fill, ${varSControlsMedia})`;
                  s.gap = "0.1cm";
                },
              }),
            ],
            children_: args.children,
          },
        ),
      };
    };

  presentation.leafChatEditorMediaSquare =
    /** @type { Presentation["leafChatEditorMediaSquare"] } */ (args) => {
      const del = e(
        "button",
        { title: "Delete" },
        {
          styles_: [
            s("leaf_chat_editor_media_delete", {
              "": (s) => {
                s.position = "absolute";
                s.top = "0.05cm";
                s.right = "0.05cm";
                s.display = "grid";
                s.justifyItems = "center";
                s.alignItems = "center";
                s.width = s.height = "0.55cm";
                s.borderRadius = "999cm";
                s.backgroundColor = `color-mix(in srgb, ${varCBackground} 80%, transparent)`;
              },
            }),
          ],
          children_: [leafSvg({ text: svgIconTrash, width: "0.4cm" })],
        },
      );
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              mediaSquareStyle,
              s("leaf_chat_editor_media_square", {
                "": (s) => {
                  s.width = s.height = varSControlsMedia;
                  s.cursor = "pointer";
                },
                ":hover": (s) => {
                  s.backgroundColor = varCBackground;
                },
              }),
            ],
            children_: [args.content, del],
          },
        ),
        delete: del,
      };
    };

  presentation.leafChatEditorFile =
    /** @type { Presentation["leafChatEditorFile"] } */ (args) => {
      const name = e(
        "input",
        { type: "text", value: args.name, placeholder: "File name" },
        {
          styles_: [
            chatEditorTextStyle,
            s("leaf_chat_editor_file_name", {
              "": (s) => {
                s.flexGrow = "1";
                s.minWidth = "0";
              },
            }),
          ],
        },
      );
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              contHboxStyle,
              s("leaf_chat_editor_file", {
                "": (s) => {
                  s.gap = "0.2cm";
                  s.alignItems = "center";
                },
              }),
            ],
            children_: [
              leafSvg({
                text: svgIconFile,
                width: varSChatControlsButton,
                extraStyles: [
                  s("leaf_chat_editor_file_icon", {
                    "": (s) => {
                      s.color = varCForegroundVeryLight;
                    },
                  }),
                ],
              }),
              name,
              e(
                "span",
                { textContent: args.size },
                {
                  styles_: [
                    s("leaf_chat_editor_file_size", {
                      "": (s) => {
                        s.color = varCForegroundLight;
                        s.fontSize = "9pt";
                      },
                    }),
                  ],
                },
              ),
            ],
          },
        ),
        name: name,
      };
    };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: message history

  const varCDiffRemoved = vs(
    "c-diff-removed",
    "rgb(255, 228, 156)",
    "rgb(120, 96, 30)",
  );
  const varCDiffAdded = vs("c-diff-added", "rgb(163, 211, 255)", "rgb(30, 80, 130)");
  const messageHistoryDiffChangedStyle = "message_history_diff_changed";

  presentation.contPageMessageHistory =
    /** @type { Presentation["contPageMessageHistory"] } */ (args) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: [contVboxStyle, ...nonchatPageStylesOuter],
            children_: [
              args.headBar,
              e(
                "div",
                {},
                {
                  styles_: [
                    nonchatPageStyleInner,
                    contVboxStyle,
                    s("cont_page_message_history_inner", {
                      "": (s) => {
                        s.width = varSPageNarrow;
                        s.alignSelf = "center";
                        s.gap = "0.5cm";
                      },
                    }),
                  ],
                  children_: args.children,
                },
              ),
            ],
          },
        ),
      };
    };

  const messageHistoryRevision =
    /** @type { (date: string, body: HTMLElement) => HTMLElement } */ (
      date,
      body,
    ) => {
      return e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            s("message_history_revision", {
              "": (s) => {
                s.gap = "0.1cm";
              },
            }),
          ],
          children_: [
            e(
              "time",
              {
                dateTime: date,
                textContent: new Date(date).toLocaleString(),
              },
              {
                styles_: [
                  s("message_history_revision_stamp", {
                    "": (s) => {
                      s.fontSize = "9pt";
                      s.fontWeight = "600";
                    },
                  }),
                ],
              },
            ),
            body,
          ],
        },
      );
    };

  const messageHistoryTextStyle = s("message_history_text", {
    "": (s) => {
      s.whiteSpace = "pre-wrap";
      s.overflowWrap = "anywhere";
      s.pointerEvents = "initial";
    },
  });

  presentation.contMessageHistoryRevisionOriginal =
    /** @type { Presentation["contMessageHistoryRevisionOriginal"] } */ (
      args,
    ) => {
      return {
        root: messageHistoryRevision(
          args.date,
          e(
            "div",
            { textContent: args.text },
            {
              styles_: [
                bubbleStyle,
                messageHistoryTextStyle,
                s("message_history_original", {
                  "": (s) => {
                    s.border = `${varLThin} solid ${varCForegroundUltraLight}`;
                  },
                }),
              ],
            },
          ),
        ),
      };
    };

  presentation.contMessageHistoryRevisionEdit =
    /** @type { Presentation["contMessageHistoryRevisionEdit"] } */ (args) => {
      return {
        root: messageHistoryRevision(
          args.date,
          e(
            "div",
            {},
            {
              styles_: [
                s("message_history_diff", {
                  "": (s) => {
                    s.display = "grid";
                    s.gridTemplateColumns = "minmax(0, 1fr) minmax(0, 1fr)";
                    s.columnGap = varPSmall;
                    s.rowGap = "0.1cm";
                  },
                }),
              ],
              children_: args.cells,
            },
          ),
        ),
      };
    };

  const messageHistoryDiffLineStyle = s("message_history_diff_line", {
    "": (s) => {
      s.borderLeft = `0.12cm solid transparent`;
      s.borderRadius = varRSelectable;
      s.padding = "0.05cm 0.15cm";
    },
  });

  presentation.contMessageHistoryDiffLineContext =
    /** @type { Presentation["contMessageHistoryDiffLineContext"] } */ (
      args,
    ) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              messageHistoryDiffLineStyle,
              messageHistoryTextStyle,
              s("message_history_diff_line_context", {
                "": (s) => {
                  s.borderLeftColor = varCForegroundUltraLight;
                  s.color = varCForegroundLight;
                },
              }),
            ],
            children_: args.children,
          },
        ),
      };
    };

  presentation.contMessageHistoryDiffLineRemoved =
    /** @type { Presentation["contMessageHistoryDiffLineRemoved"] } */ (
      args,
    ) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              messageHistoryDiffLineStyle,
              messageHistoryTextStyle,
              s("message_history_diff_line_removed", {
                "": (s) => {
                  s.borderLeftColor = varCDiffRemoved;
                  s.backgroundColor = `color-mix(in srgb, ${varCDiffRemoved} 25%, transparent)`;
                },
                [` .${messageHistoryDiffChangedStyle}`]: (s) => {
                  s.backgroundColor = varCDiffRemoved;
                },
              }),
            ],
            children_: args.children,
          },
        ),
      };
    };

  presentation.contMessageHistoryDiffLineAdded =
    /** @type { Presentation["contMessageHistoryDiffLineAdded"] } */ (args) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              messageHistoryDiffLineStyle,
              messageHistoryTextStyle,
              s("message_history_diff_line_added", {
                "": (s) => {
                  s.borderLeftColor = varCDiffAdded;
                  s.backgroundColor = `color-mix(in srgb, ${varCDiffAdded} 25%, transparent)`;
                },
                [` .${messageHistoryDiffChangedStyle}`]: (s) => {
                  s.backgroundColor = varCDiffAdded;
                },
              }),
            ],
            children_: args.children,
          },
        ),
      };
    };

  presentation.leafMessageHistoryDiffLineEmpty =
    /** @type { Presentation["leafMessageHistoryDiffLineEmpty"] } */ (args) => {
      return { root: e("div", {}, {}) };
    };

  presentation.leafMessageHistoryDiffText =
    /** @type { Presentation["leafMessageHistoryDiffText"] } */ (args) => {
      return {
        root: e(
          "span",
          { textContent: args.text },
          {
            styles_: args.changed
              ? [
                  messageHistoryDiffChangedStyle,
                  s("message_history_diff_text_changed", {
                    "": (s) => {
                      s.borderRadius = "0.05cm";
                      s.fontWeight = "600";
                    },
                  }),
                ]
              : [],
          },
        ),
      };
    };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: events

  presentation.leafChatHeadBarRightCalendar =
    /** @type { Presentation["leafChatHeadBarRightCalendar"] } */ (args) => {
      return { root: headIconButton({ link: args.link, svg: svgIconCalendar }) };
    };

  presentation.leafChatHeadBarRightMessages =
    /** @type { Presentation["leafChatHeadBarRightMessages"] } */ (args) => {
      return { root: headIconButton({ link: args.link, svg: svgIconMessage }) };
    };

  const eventTimelineStyle = s("event_timeline", {
    "": (s) => {
      s.display = "flex";
      s.flexDirection = "row";
      s.alignItems = "center";
      s.gap = varPSmall;
      s.minWidth = "0";
      s.alignSelf = "stretch";
      s.color = varCForegroundLight;
    },
    ">span": (s) => {
      s.minWidth = "0";
      s.flexShrink = "1";
    },
    ">svg": (s) => {
      s.color = varCForegroundVeryLight;
    },
  });
  const eventTimelineChildren = /** @type { (text: string) => HTMLElement[] } */ (
    text,
  ) => {
    return [
      leafSvg({ text: svgIconCalendar, width: varSChatControlsButton }),
      e("span", { textContent: text }, {}),
    ];
  };

  presentation.leafChatEntryMessageEvent =
    /** @type { Presentation["leafChatEntryMessageEvent"] } */ (args) => {
      return {
        root: e("div", {}, {
          styles_: [
            eventTimelineStyle,
            s("leaf_chat_entry_message_event", {
              "": (s) => {
                s.pointerEvents = "initial";
              },
            }),
          ],
          children_: eventTimelineChildren(args.timeline),
        }),
      };
    };

  presentation.leafChatEditorToolboxButtonEvent =
    /** @type { Presentation["leafChatEditorToolboxButtonEvent"] } */ (args) => {
      return { root: chatControlsIconButton(svgIconCalendar, "Add event") };
    };

  presentation.leafChatEditorEvent =
    /** @type { Presentation["leafChatEditorEvent"] } */ (args) => {
      return {
        root: e(
          "div",
          { title: "Edit event time", tabIndex: 0, role: "button" },
          {
            styles_: [eventTimelineStyle, onwhiteSelectableStyle],
            children_: eventTimelineChildren(args.timeline),
          },
        ),
      };
    };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: calendar

  const varSCalendar = "10cm";
  const varSCalendarMark = `calc(${varSUnread} * 2 / 3)`;
  const varSCalendarWidth = `min(100cqw - 2 * ${varPPage}, ${varSCalendar})`;
  const varSCalendarCellHeight = `calc((${varSCalendarWidth} - 6 * ${varPSmall}) / 7 * 4 / 3)`;

  presentation.contCalendarMonth =
    /** @type { Presentation["contCalendarMonth"] } */ (args) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              s("cont_calendar_month", {
                "": (s) => {
                  s.display = "grid";
                  s.gridTemplateColumns = "repeat(7, minmax(0, 1fr))";
                  s.gap = varPSmall;
                  s.width = varSCalendarWidth;
                  s.marginLeft = "auto";
                  s.marginRight = "auto";
                  s.pointerEvents = "none";
                  s.position = "relative";
                },
              }),
              args.overlapNext
                ? s("cont_calendar_month_overlap", {
                    "": (s) => {
                      s.marginBottom = `calc(-0.5 * ${varSCalendarCellHeight})`;
                    },
                  })
                : s("cont_calendar_month_no_overlap", {
                    "": (s) => {
                      s.marginBottom = `calc(0.5 * ${varSCalendarCellHeight})`;
                    },
                  }),
            ],
            children_: args.children,
          },
        ),
      };
    };

  presentation.leafCalendarDay =
    /** @type { Presentation["leafCalendarDay"] } */ (args) => {
      const styles = [
        s("leaf_calendar_day", {
          "": (s) => {
            s.position = "relative";
            s.aspectRatio = "3 / 4";
            s.border = `${varLVeryThin} solid ${varCForeground}`;
            s.overflow = "hidden";
            s.display = "grid";
            s.justifyItems = "center";
            s.alignItems = "center";
            s.fontSize = varFMenu;
            s.fontWeight = varWHead;
            s.color = varCForeground;
            s.pointerEvents = "initial";
            s.cursor = "pointer";
          },
          ":hover": (s) => {
            s.outline = `${varLThin} solid ${varCNotifyBright}`;
            s.zIndex = "1";
          },
        }),
      ];
      if (!args.past) {
        styles.push(
          s("leaf_calendar_day_future", {
            "": (s) => {
              s.backgroundColor = "white";
            },
          }),
        );
      }
      if (args.first) {
        styles.push(
          s("leaf_calendar_day_first", {
            "": (s) => {
              s.borderTopLeftRadius = varRBubble;
            },
          }),
        );
      }
      if (args.last) {
        styles.push(
          s("leaf_calendar_day_last", {
            "": (s) => {
              s.borderBottomRightRadius = varRBubble;
            },
          }),
        );
      }
      const children = [];
      if (args.bar) {
        children.push(
          e(
            "div",
            {},
            {
              styles_: [
                s("leaf_calendar_day_bar", {
                  "": (s) => {
                    s.position = "absolute";
                    s.left = "0";
                    s.right = "0";
                    s.top = "0";
                    s.height = `calc(${varSCalendarMark} / 2)`;
                    s.backgroundColor = varCNotifyBright;
                  },
                }),
              ],
            },
          ),
        );
      }
      if (args.dot) {
        children.push(
          e(
            "div",
            {},
            {
              styles_: [
                s("leaf_calendar_day_dot", {
                  "": (s) => {
                    s.position = "absolute";
                    s.right = varPIconButton;
                    s.bottom = varPIconButton;
                    s.width = varSCalendarMark;
                    s.height = varSCalendarMark;
                    s.borderRadius = "999cm";
                    s.backgroundColor = varCNotifyBright;
                  },
                }),
              ],
            },
          ),
        );
      }
      children.push(e("span", { textContent: args.day }, {}));
      const out = e("div", {}, { styles_: styles, children_: children });
      if (args.column != null) {
        out.style.gridColumnStart = args.column;
      }
      return { root: out };
    };

  const calendarStackStyle = s("cont_calendar_stack", {
    "": (s) => {
      s.display = "grid";
      s.gridTemplateColumns = "minmax(0, 1fr)";
      s.gridTemplateRows = "minmax(0, 1fr)";
      s.flexGrow = "1";
      s.minHeight = "0";
      s.maxHeight = "100%";
    },
    ">*": (s) => {
      s.gridColumn = "1";
      s.gridRow = "1";
      s.minHeight = "0";
      s.minWidth = "0";
    },
    ">*:not(:last-child)": (s) => {
      s.display = "none";
    },
  });
  presentation.contCalendarStack =
    /** @type { Presentation["contCalendarStack"] } */ (args) => {
      return { root: e("div", {}, { styles_: [calendarStackStyle] }) };
    };

  presentation.contCalendarModal =
    /** @type { Presentation["contCalendarModal"] } */ (args) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              calendarStackStyle,
              s("cont_calendar_modal", {
                "": (s) => {
                  s.backgroundColor = varCBackground;
                  s.pointerEvents = "initial";
                  s.position = "absolute";
                  s.inset = "0";
                  s.zIndex = "1000";
                  s.overflowY = "auto";
                },
              }),
            ],
          },
        ),
      };
    };

  presentation.leafCalendarHeadBarButtonSelectCalendar =
    /** @type { Presentation["leafCalendarHeadBarButtonSelectCalendar"] } */ (
      args,
    ) => {
      return {
        root: e(
          "button",
          { title: "Select calendar" },
          {
            styles_: [...onBlueIconButtonStyles],
            children_: [
              leafSvg({
                text: svgIconCalendar,
                width: varSHeadButton,
                extraStyles: [
                  s("head_icon_button", {
                    "": (s) => {
                      s.color = varCForegroundVeryLight;
                    },
                  }),
                ],
              }),
            ],
          },
        ),
      };
    };

  presentation.contCalendarPage =
    /** @type { Presentation["contCalendarPage"] } */ (args) => {
      const children = [args.headBar, args.body];
      if (args.controls != null) {
        children.push(args.controls);
      }
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              contVboxStyle,
              classPageChat,
              s("cont_calendar_page", {
                "": (s) => {
                  s.minHeight = "0";
                  s.maxHeight = "100%";
                  s.overflow = "hidden";
                },
                ">*:nth-child(2)": (s) => {
                  s.flexGrow = "1";
                  s.flexBasis = "0";
                  s.minHeight = "0";
                },
              }),
            ],
            children_: children,
          },
        ),
      };
    };

  presentation.contCalendarDayList =
    /** @type { Presentation["contCalendarDayList"] } */ (args) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              contVboxStyle,
              s("cont_calendar_day_list", {
                "": (s) => {
                  s.overflowY = "auto";
                },
              }),
            ],
            children_: args.children,
          },
        ),
      };
    };

  presentation.leafCalendarDayEmpty =
    /** @type { Presentation["leafCalendarDayEmpty"] } */ (args) => {
      return {
        root: e(
          "div",
          { textContent: "No events" },
          {
            styles_: [
              s("leaf_calendar_day_empty", {
                "": (s) => {
                  s.alignSelf = "center";
                  s.color = varCForegroundVeryLight;
                  s.padding = varPPage;
                },
              }),
            ],
          },
        ),
      };
    };

  presentation.leafCalendarDayButtonGoTo =
    /** @type { Presentation["leafCalendarDayButtonGoTo"] } */ (args) => {
      return { root: chatControlsIconButton(svgIconForward, "Go to message") };
    };

  presentation.leafCalendarDayButtonSelect =
    /** @type { Presentation["leafCalendarDayButtonSelect"] } */ (args) => {
      return { root: chatControlsIconButton(svgIconCheck, "Select day") };
    };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: modals

  globalStyle.insertRule(`.${classStateHidden} { display: none !important; }`, 0);
  globalStyle.insertRule(`.malarkdowney { white-space-collapse: preserve; }`, 0);
  globalStyle.insertRule(
    `code.malarkdowney, .malarkdowney_block_code { font-family: monospace; }`,
    0,
  );
  globalStyle.insertRule(`.malarkdowney_line { min-height: 1em; }`, 0);
  globalStyle.insertRule(`codeline { display: block; }`, 0);
  globalStyle.insertRule(
    `.malarkdowney_aligned { position: relative; display: inline-block; width: 0; text-wrap: nowrap; }`,
    0,
  );
  globalStyle.insertRule(`.malarkdowney a { pointer-events: initial; text-decoration: underline; }`, 0);

  presentation.contModalRoot = /** @type { Presentation["contModalRoot"] } */ (
    args,
  ) => {
    return {
      root: e(
        "div",
        {},
        {
          styles_: [
            contGroupStyle,
            s("cont_modal_root", {
              ">*:not(:last-child)": (s) => {
                s.display = "none";
              },
            }),
          ],
        },
      ),
    };
  };

  const modalOverlayStyle = s("modal_overlay", {
    "": (s) => {
      s.position = "fixed";
      s.inset = "0";
      s.zIndex = "1000";
      s.pointerEvents = "initial";
    },
  });
  const modalTextButtonStyle = s("modal_text_button", {
    "": (s) => {
      s.padding = "0.2cm 0.4cm";
      s.borderRadius = varRInput;
      s.backgroundColor = `color-mix(in srgb, ${varCBackground} 80%, transparent)`;
      s.color = varCForeground;
    },
    ":hover": (s) => {
      s.backgroundColor = varCBackground;
    },
  });
  const modalTextButton = /** @type { (text: string) => HTMLElement } */ (
    text,
  ) => {
    return e("button", { textContent: text }, { styles_: [modalTextButtonStyle] });
  };
  const modalIconButtonStyle = s("modal_icon_button", {
    "": (s) => {
      s.display = "grid";
      s.justifyItems = "center";
      s.alignItems = "center";
      s.width = s.height = "1cm";
      s.borderRadius = "999cm";
      s.color = "white";
      s.backgroundColor = `color-mix(in srgb, black 40%, transparent)`;
    },
    ":hover": (s) => {
      s.backgroundColor = `color-mix(in srgb, white 30%, transparent)`;
    },
    [`.${classStateSelected}`]: (s) => {
      s.backgroundColor = `color-mix(in srgb, white 30%, transparent)`;
    },
    [`.${classStateSelected}:hover`]: (s) => {
      s.backgroundColor = `color-mix(in srgb, white 15%, transparent)`;
    },
  });
  const modalIconButton = /** @type { (svg: string, title: string, tag?: "button"|"a") => HTMLElement } */ (
    svg,
    title,
    tag,
  ) => {
    return e(
      tag ?? "button",
      { title: title },
      {
        styles_: [modalIconButtonStyle],
        children_: [leafSvg({ text: svg, width: "0.6cm" })],
      },
    );
  };
  const modalDarkStyle = s("modal_dark", {
    "": (s) => {
      s.backgroundColor = "black";
      s.color = "white";
      s.display = "grid";
      s.gridTemplateColumns = "1fr";
      s.gridTemplateRows = "1fr";
    },
  });
  const modalFillStyle = s("modal_fill", {
    "": (s) => {
      s.gridColumn = "1";
      s.gridRow = "1";
      s.width = "100%";
      s.height = "100%";
      s.minWidth = "0";
      s.minHeight = "0";
      s.objectFit = "contain";
    },
  });
  const modalBarStyle = s("modal_bar", {
    "": (s) => {
      s.gridColumn = "1";
      s.gridRow = "1";
      s.display = "flex";
      s.flexDirection = "row";
      s.alignItems = "center";
      s.justifyContent = "space-between";
      s.gap = "0.3cm";
      s.padding = "0.3cm";
      s.zIndex = "1";
    },
  });

  presentation.contChatEditorModalConfirm =
    /** @type { Presentation["contChatEditorModalConfirm"] } */ (args) => {
      const ok = modalTextButton("Delete");
      const cancel = modalTextButton("Cancel");
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              modalOverlayStyle,
              s("chat_editor_modal_confirm", {
                "": (s) => {
                  s.display = "grid";
                  s.justifyItems = "center";
                  s.alignItems = "center";
                  s.backgroundColor = `color-mix(in srgb, black 40%, transparent)`;
                },
              }),
            ],
            children_: [
              e(
                "div",
                {},
                {
                  styles_: [
                    contVboxStyle,
                    bubbleStyle,
                    s("chat_editor_modal_confirm_panel", {
                      "": (s) => {
                        s.gap = "0.4cm";
                        s.padding = "0.5cm";
                        s.backgroundColor = varCBackground;
                        s.maxWidth = varSPageNarrow;
                      },
                    }),
                  ],
                  children_: [
                    e("p", { textContent: args.text }, {}),
                    e(
                      "div",
                      {},
                      {
                        styles_: [
                          contHboxStyle,
                          s("chat_editor_modal_confirm_buttons", {
                            "": (s) => {
                              s.justifyContent = "flex-end";
                              s.gap = "0.2cm";
                            },
                          }),
                        ],
                        children_: [cancel, ok],
                      },
                    ),
                  ],
                },
              ),
            ],
          },
        ),
        ok: ok,
        cancel: cancel,
      };
    };

  presentation.contChatModalMediaViewer =
    /** @type { Presentation["contChatModalMediaViewer"] } */ (args) => {
      const stage = e(
        "div",
        {},
        {
          styles_: [
            modalFillStyle,
            s("chat_modal_media_viewer_stage", {
              "": (s) => {
                s.display = "grid";
                s.gridTemplateColumns = "1fr";
                s.gridTemplateRows = "1fr";
                s.justifyItems = "center";
                s.alignItems = "center";
              },
            }),
          ],
        },
      );
      const download = modalIconButton(svgIconDownload, "Download", "a");
      const close = modalIconButton(svgIconNope, "Close");
      const prev = modalIconButton(svgIconBack, "Previous");
      const next = modalIconButton(svgIconForward, "Next");
      const counter = e("span", {}, {});
      const controlsStyle = s("chat_modal_media_viewer_controls", {
        "": (s) => {
        },
      });
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              modalOverlayStyle,
              modalDarkStyle,
              s("chat_modal_media_viewer", {
                [`.${classStateControlsHidden}`]: (s) => {
                  s.cursor = "none";
                },
                [`.${classStateControlsHidden} .${controlsStyle}`]: (s) => {
                  s.opacity = "0";
                  s.pointerEvents = "none";
                },
              }),
            ],
            children_: [
              stage,
              e(
                "div",
                {},
                {
                  styles_: [
                    modalBarStyle,
                    controlsStyle,
                    s("chat_modal_media_viewer_top", {
                      "": (s) => {
                        s.alignSelf = "start";
                      },
                    }),
                  ],
                  children_: [
                    counter,
                    e(
                      "div",
                      {},
                      {
                        styles_: [
                          contHboxStyle,
                          s("chat_modal_media_viewer_top_buttons", {
                            "": (s) => {
                              s.gap = "0.3cm";
                            },
                          }),
                        ],
                        children_: [download, close],
                      },
                    ),
                  ],
                },
              ),
              e(
                "div",
                {},
                {
                  styles_: [
                    modalBarStyle,
                    controlsStyle,
                    s("chat_modal_media_viewer_sides", {
                      "": (s) => {
                        s.alignSelf = "center";
                        s.pointerEvents = "none";
                      },
                      " > *": (s) => {
                        s.pointerEvents = "initial";
                      },
                    }),
                  ],
                  children_: [prev, next],
                },
              ),
            ],
          },
        ),
        stage: stage,
        download: download,
        close: close,
        prev: prev,
        next: next,
        counter: counter,
      };
    };

  presentation.leafChatModalMediaViewerImage =
    /** @type { Presentation["leafChatModalMediaViewerImage"] } */ (args) => {
      return { root: e("img", { src: args.src }, { styles_: [modalFillStyle] }) };
    };

  presentation.leafChatModalMediaViewerVideo =
    /** @type { Presentation["leafChatModalMediaViewerVideo"] } */ (args) => {
      return {
        root: e(
          "video",
          { src: args.src, controls: true, autoplay: true, playsInline: true },
          {
            styles_: [
              modalFillStyle,
              s("chat_modal_media_viewer_video", {
                "": (s) => {
                  s.pointerEvents = "initial";
                },
              }),
            ],
          },
        ),
      };
    };

  presentation.contChatEditorModalCamera =
    /** @type { Presentation["contChatEditorModalCamera"] } */ (args) => {
      const video = e(
        "video",
        { autoplay: true, muted: true, playsInline: true },
        { styles_: [modalFillStyle] },
      );
      const status = e(
        "p",
        {},
        {
          styles_: [
            s("chat_editor_modal_camera_status", {
              "": (s) => {
                s.gridColumn = "1";
                s.gridRow = "1";
                s.placeSelf = "center";
                s.padding = "0.5cm";
              },
            }),
          ],
        },
      );
      const close = modalIconButton(svgIconNope, "Close");
      const flip = modalIconButton(svgIconFlip, "Switch camera");
      const modePhoto = modalIconButton(svgIconCamera, "Photo");
      const modeVideo = modalIconButton(svgIconVideo, "Video");
      const shutterInner = s("chat_editor_modal_camera_shutter_inner", {
        "": (s) => {
          s.width = s.height = "1.2cm";
          s.borderRadius = "999cm";
          s.backgroundColor = "white";
        },
      });
      const shutter = e(
        "button",
        { title: "Capture" },
        {
          styles_: [
            s("chat_editor_modal_camera_shutter", {
              "": (s) => {
                s.display = "grid";
                s.justifyItems = "center";
                s.alignItems = "center";
                s.width = s.height = "1.6cm";
                s.borderRadius = "999cm";
                s.border = "0.1cm solid white";
              },
              [`:hover .${shutterInner}`]: (s) => {
                s.opacity = varOIcon;
              },
              [`.${classStateVideo} .${shutterInner}`]: (s) => {
                s.backgroundColor = "rgb(220, 40, 40)";
              },
              [`.${classStateRecording} .${shutterInner}`]: (s) => {
                s.width = s.height = "0.6cm";
                s.borderRadius = "0.1cm";
              },
            }),
          ],
          children_: [e("div", {}, { styles_: [shutterInner] })],
        },
      );
      return {
        root: e(
          "div",
          {},
          {
            styles_: [modalOverlayStyle, modalDarkStyle],
            children_: [
              video,
              status,
              e(
                "div",
                {},
                {
                  styles_: [
                    modalBarStyle,
                    s("chat_editor_modal_camera_top", {
                      "": (s) => {
                        s.alignSelf = "start";
                      },
                    }),
                  ],
                  children_: [close, flip],
                },
              ),
              e(
                "div",
                {},
                {
                  styles_: [
                    modalBarStyle,
                    s("chat_editor_modal_camera_bottom", {
                      "": (s) => {
                        s.alignSelf = "end";
                        s.justifyContent = "center";
                      },
                    }),
                  ],
                  children_: [modePhoto, shutter, modeVideo],
                },
              ),
            ],
          },
        ),
        video: video,
        status: status,
        close: close,
        flip: flip,
        modePhoto: modePhoto,
        modeVideo: modeVideo,
        shutter: shutter,
      };
    };

  presentation.contChatEditorModalCameraPreview =
    /** @type { Presentation["contChatEditorModalCameraPreview"] } */ (args) => {
      const retake = modalIconButton(svgIconTrash, "Retake");
      const accept = modalIconButton(svgIconCheck, "Use");
      /** @type { HTMLElement[] } */
      const bar = [retake];
      if (args.options != null) {
        bar.push(args.options);
      }
      bar.push(accept);
      return {
        root: e(
          "div",
          {},
          {
            styles_: [modalOverlayStyle, modalDarkStyle],
            children_: [
              args.media,
              e(
                "div",
                {},
                {
                  styles_: [
                    modalBarStyle,
                    s("chat_editor_modal_camera_preview_bottom", {
                      "": (s) => {
                        s.alignSelf = "end";
                      },
                    }),
                  ],
                  children_: bar,
                },
              ),
            ],
          },
        ),
        retake: retake,
        accept: accept,
      };
    };

  presentation.leafChatEditorModalCameraPreviewImage =
    /** @type { Presentation["leafChatEditorModalCameraPreviewImage"] } */ (args) => {
      return { root: e("img", { src: args.src }, { styles_: [modalFillStyle] }) };
    };

  presentation.leafChatEditorModalCameraPreviewVideo =
    /** @type { Presentation["leafChatEditorModalCameraPreviewVideo"] } */ (args) => {
      return {
        root: e(
          "video",
          { src: args.src, controls: true, autoplay: true, loop: true, playsInline: true },
          {
            styles_: [
              modalFillStyle,
              s("chat_editor_modal_camera_preview_video", {
                "": (s) => {
                  s.pointerEvents = "initial";
                },
              }),
            ],
          },
        ),
      };
    };

  presentation.leafChatEditorModalCameraPreviewSelect =
    /** @type { Presentation["leafChatEditorModalCameraPreviewSelect"] } */ (args) => {
      return {
        root: e(
          "select",
          {},
          {
            styles_: [
              modalTextButtonStyle,
              s("chat_editor_modal_camera_preview_select", {
                "": (s) => {
                  s.flexShrink = "1";
                  s.minWidth = "0";
                },
              }),
            ],
            children_: args.children,
          },
        ),
      };
    };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Assemble

  window.kwaPresentation = presentation;

  addEventListener("DOMContentLoaded", (_) => {
    const resetStyle = e(
      "link",
      { rel: "stylesheet", href: new URL("style_reset.css", scriptUrl).href },
      {},
    );
    document.head.appendChild(resetStyle);
    notnull(document.body).classList.add(
      s("body", {
        "": (s) => {
          s.fontFamily = "X";
          //s.backgroundColor = varCBackground;
          s.background = varCBackground;
          s.color = varCForeground;
        },
      }),
    );
    document.body.classList.add(contStackStyle);
  });
}
