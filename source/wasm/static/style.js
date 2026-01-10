/// <reference path="style_export.d.ts" />
/// <reference path="style_export2.d.ts" />
{
  const presentation = {};

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
    dark
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
                .style
            );
            break;
          case "wide":
            globalStyleMediaWide.insertRule(`.${id1}${suffix} {}`, 0);
            f1(
              /** @type { CSSStyleRule } */ (globalStyleMediaWide.cssRules[0])
                .style
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

  const textIconDelete = "\ue15b";
  const textIconAdd = "\ue145";
  const textIconAddMessage = "\ue266";
  const textIconSettings = "\ue8b8";
  const textIconIdentities = "\ue7fd";
  const textIconContacts = "\uf4ca";
  const textIconBack = "\ue2ea";
  const textIconLink = "\ue157";
  const textIconCopy = "\ue14d";
  const textIconLogin = "\uea77";
  const textIconLogout = "\ue9ba";
  const textIconFoldArrow = "\ue5e1";
  const textIconClose = "\ue5cd";
  const textIconSend = "\ue163";
  const textIconEdit = "\ue3c9";

  // xx Variables
  const varCBackground = vs(
    "background",
    //"rgb(98, 102, 119)",
    "rgb(176, 180, 196)",
    "rgb(70, 73, 77)"
  );
  //const varCBackgroundGrad2 = vs("background-end", "#9ab0c4", "rgb(0,0,0)");
  const varCBackgroundGrad2 = varCBackground;
  const varCBackgroundBubble = vs(
    "background-bubble",
    "whitesmoke",
    "rgb(70, 73, 77)"
  );
  const varCBackgroundBubble2 = vs(
    "background-bubble2",
    "rgba(224, 212, 200, 1)",
    "rgb(70, 73, 77)"
  );
  const varCForeground = vs(
    "c-foreground",
    "rgb(30, 30, 30)",
    "rgb(244, 255, 255)"
  );
  const varCForegroundLight = `color-mix(in srgb, ${varCForeground} 80%, transparent)`;
  const varCForegroundVeryLight = `color-mix(in srgb, ${varCForeground} 50%, transparent)`;
  const varCForegroundUltraLight = `color-mix(in srgb, ${varCForeground} 20%, transparent)`;
  const varCForegroundHeadCenter = `color-mix(in srgb, ${varCForeground} 50%, ${varCBackground})`;
  const varCMutateForeground = "white";
  const varCMutateBackground = vs(
    "c-foreground-mutate",
    //"rgb(164, 32, 73)",
    //"rgb(137, 81, 219)",
    //"rgba(76, 76, 146, 1)",
    "rgba(120, 66, 87, 1)",
    "rgb(0,0,0)"
  );
  const varCNotifyForeground = "white";
  const varCNotifyBackground = vs(
    "c-foreground-notify",
    //"rgb(164, 32, 73)",
    "rgba(68, 78, 111, 1)",
    "rgb(0,0,0)"
  );
  const varCNotifyBright = vs(
    "c-foreground-notify-bright",
    //"rgb(164, 32, 73)",
    "rgba(81, 101, 176, 1)",
    "rgb(0,0,0)"
  );
  const varCForegroundError = vs(
    "c-foreground-error",
    "rgb(154, 60, 74)",
    "rgb(243, 69, 95)"
  );

  const varFMenu = "20pt";
  const varFHeadBar = "13pt";

  const varSHeadButton = "0.8cm";
  const varSPageNarrow = "min(100%, 20cm)";
  const varSChatEntry = "16cm";
  const varSPortrait = "1.5cm";
  const varSChatControlsButton = "0.8cm";
  const varSMenuIcon = "1cm";
  const varSMenuIconShiftCalc = `(${varSMenuIcon} + 0.15cm)`;

  const varPPage = "0.3cm";
  const varPSmall = "0.2cm";
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

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: all

  const contGroupStyle = "group";
  const contVboxStyle = "vbox";
  const contHboxStyle = "hbox";
  const contStackStyle = "stack";

  presentation.contGroup = /** @type {Presentation["contGroup"]} */ (args) => ({
    root: e("div", {}, { styles_: [contGroupStyle], children_: args.children }),
  });

  const menuItemLineHeight = "1cm";
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
    `)
    );
  });
  const leafSpinner = /** @type { (extraStyles: string[])=>HTMLElement } */ (
    extraStyles
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
        " "
      )} spinner1">
        <path fill="none" stroke="currentColor" stroke-width="${varLAsyncSvg}" pathLength="1" d="m 0.04276819,0.30609795 c 0.02501087,-0.0621192 0.02587843,-0.26140339 0.07010972,-0.25775973 0.0442308,0.0036438 0.0392081,0.24571051 0.0949588,0.24268559 0.0557507,-0.003024 0.044678,-0.19950842 0.10249788,-0.1955405 0.0578198,0.0039683 0.0658219,0.15775144 0.13449927,0.15586294 0.0686774,-0.001889 0.10746733,-0.0981084 0.17282507,-0.095961 0.0653576,0.002148 0.11054793,0.0614271 0.1828686,0.0649971 0.0723208,0.00357 0.0987762,-0.0464728 0.16302072,-0.0447597" />
      </svg>
    `
      );
    } else if (sel < 0.5) {
      return et(
        `
      <svg viewBox="0 0 1 0.35" xmlns="http://www.w3.org/2000/svg" class="${styles.join(
        " "
      )} spinner2">
        <path fill="none" stroke="currentColor" stroke-width="${varLAsyncSvg}" pathLength="1" d="M 0.14254063,0.2938125 C 0.10301621,0.2571751 0.0885351,0.17576883 0.12872349,0.11336434 c 0.0401883,-0.06240458 0.17293408,-0.08274137 0.25797572,-0.003932 0.0850417,0.0788098 0.0269571,0.13671136 -0.039893,0.13923062 -0.0668499,0.002519 -0.0812133,-0.0627161 -0.0300127,-0.0891682 0.0512009,-0.0264521 0.15399772,0.020863 0.23776893,0.0435695 0.0837712,0.0227066 0.19943644,0.028629 0.27049542,4.1993e-4 0.0710589,-0.0282097 0.0922414,-0.0677874 0.0456478,-0.0856104" />
      </svg>
    `
      );
    } else if (sel < 0.75) {
      return et(
        `
      <svg viewBox="0 0 1 0.35" xmlns="http://www.w3.org/2000/svg" class="${styles.join(
        " "
      )} spinner3">
        <path fill="none" stroke="currentColor" stroke-width="${varLAsyncSvg}" pathLength="1" d="M 0.05805971,0.13660498 C 0.10797019,0.26193567 0.35525482,0.19161688 0.38586466,0.11202727 0.41647444,0.03243767 0.3367885,0.0220047 0.31256276,0.08803155 0.28833702,0.1540584 0.27688391,0.22803629 0.38859489,0.24382805 0.50030587,0.2596199 0.66125404,0.11789295 0.71967761,0.22511469 0.77810135,0.3323364 0.59090405,0.3361921 0.60743783,0.21194022 0.62397152,0.08768833 0.8381139,0.13047976 0.94682009,0.15652352" />
      </svg>
    `
      );
    } else {
      return et(
        `
      <svg viewBox="0 0 1 0.35" xmlns="http://www.w3.org/2000/svg" class="${styles.join(
        " "
      )} spinner4">
        <path fill="none" stroke="currentColor" stroke-width="${varLAsyncSvg}" pathLength="1" d="m 0.06717016,0.19875525 c 0.04590072,0.0640954 0.09986502,0.0862412 0.14118426,0.0637398 0.0413194,-0.0225014 0.0786624,-0.0661734 0.0763519,-0.12579028 -0.002313,-0.05961708 -0.0802163,-0.07410729 -0.0724987,0.0114052 0.007718,0.0855125 0.0981388,0.10332275 0.14289414,0.10209764 0.0447553,-0.001225 0.14567377,-0.0514403 0.14452406,-0.11976192 -0.001152,-0.06832152 -0.0713155,-0.06170884 -0.077116,0.006882 -0.0058,0.0685904 0.079857,0.11707331 0.13892941,0.11242721 0.0590725,-0.004646 0.14949369,-0.0504459 0.14921933,-0.11892872 -2.7486e-4,-0.06848265 -0.0729905,-0.07196589 -0.0718564,0.006953 0.001134,0.0789188 0.0699061,0.11761026 0.12504166,0.11801568 0.0551356,4.0553e-4 0.1429375,-0.0578232 0.16859285,-0.10290166" />
      </svg>
    `
      );
    }
  };

  presentation.leafAsyncBlock = /** @type {Presentation["leafAsyncBlock"]} */ (
    args
  ) => {
    const inner = e(
      "div",
      {},
      {
        styles_: [
          contStackStyle,
          s(["leaf_async_block"], {
            "": (s) => {
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
      }
    );
    return {
      root: inner,
    };
  };

  presentation.leafErrBlock = /** @type {Presentation["leafErrBlock"]} */ (
    args
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
            }
          ),
        ],
      }
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
                s.display = "flex";
                s.flexDirection = "column";
                s.justifyContent = "stretch";
                s.height = "100dvh";
                s.overflowY = "scroll";
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
      }
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
                  s.columnGap = "0.5cm";
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

                      s.display = "flex";
                      s.flexDirection = "column";
                      s.justifyContent = "stretch";
                      s.height = "100dvh";
                      s.overflowY = "scroll";
                    },
                  }),
                ],
                children_: [args.menu],
              }
            ),
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
                        s.width = varLVeryThin;
                        s.backgroundColor = varCForegroundUltraLight;
                      },
                      narrow: (s) => {
                        s.display = "none";
                      },
                    },
                  }),
                ],
              }
            ),
            page,
          ],
        }
      ),
    };
  };
  presentation.contPageBlank = /** @type { Presentation["contPageBlank"] } */ (
    args
  ) => {
    return { root: e("div", {}, {}) };
  };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: menu, form, top

  const buttonStyle = s("button", {
    "": (s) => {},
    ":hover": (s) => {
      s.opacity = "0.5";
    },
  });
  const buttonMutStyles = [
    buttonStyle,
    s("button_mut", {
      "": (s) => {
        s.backgroundColor = varCMutateBackground;
        s.color = "white";
        s.borderRadius = "0.2cm";
      },
    }),
  ];
  const buttonMutStylesText = [
    ...buttonMutStyles,
    s("button_mut_text", {
      "": (s) => {
        s.fontFamily = "sans-serif";
        s.fontSize = "14pt";
        s.fontWeight = "500";
        s.padding = "0.2cm 0.4cm";
      },
    }),
  ];
  const menuItemStyles = [
    s("leaf_menu_item", {
      "": (s) => {
        s.fontSize = varFMenu;
        s.display = "flex";
        s.alignItems = "center";
        s.height = menuItemLineHeight;
        s.fontWeight = "600";
        s.lineHeight = menuItemLineHeight;
        s.flexGrow = "1";
        s.justifyContent = "space-between";
      },
    }),
    buttonStyle,
  ];
  const topSelectBgStyle = s("top_selectable_bg", {
    "": (s) => {
      s.display = "none";
      s.backgroundColor = varCBackgroundBubble;
      s.position = "absolute";
      s.inset = "-0.05cm";
      s.borderRadius = "0.1cm";
      s.zIndex = "-1";
    },
  });
  const topSelectableStyle = s("top_selectable", {
    "": (s) => {
      s.position = "relative";
    },
    [`.${classStateSelected} > summary .${topSelectBgStyle}`]: (s) => {
      s.display = "initial";
    },
    [`.${classStateSelected} > .${topSelectBgStyle}`]: (s) => {
      s.display = "initial";
    },
  });
  const createTopItemSelectBg = () => {
    return e("div", {}, { styles_: [topSelectBgStyle] });
  };

  presentation.leafMenuLink = /** @type { Presentation["leafMenuLink"] } */ (
    args
  ) => {
    const unread = e("div", {}, { styles_: [unreadStyle] });
    /** @type { HTMLElement[] } */
    const children = [createTopItemSelectBg()];
    if (args.image != null) {
      children.push(
        e(
          "img",
          { src: args.image },
          {
            styles_: [
              s("leaf_menu_link_icon", {
                "": (s) => {
                  s.width = varSMenuIcon;
                  s.height = varSMenuIcon;
                  s.marginLeft = `calc(-1.0 * ${varSMenuIconShiftCalc})`;
                  s.borderRadius = varRPortrait;
                },
              }),
            ],
          }
        )
      );
    }
    children.push(
      e(
        "span",
        { textContent: args.text },
        {
          styles_: [
            s("leaf_menu_link_text", {
              "": (s) => {
                s.flexBasis = "0";
                s.flexGrow = "1";
                s.textOverflow = "ellipsis";
                s.overflowX = "hidden";
                s.whiteSpace = "nowrap";
              },
            }),
          ],
        }
      )
    );
    children.push(unread);
    return {
      root: e(
        "a",
        { href: args.link },
        {
          styles_: [
            ...menuItemStyles,
            topSelectableStyle,
            s("leaf_menu_link", {
              "": (s) => {
                s.gap = "0.1cm";
                s.maxWidth = "100%";
              },
            }),
          ],
          children_: children,
        }
      ),
      unread: unread,
    };
  };

  presentation.leafMenuButton =
    /** @type { Presentation["leafMenuButton"] } */ (args) => {
      return {
        root: e(
          "button",
          { textContent: args.text },
          { styles_: [...menuItemStyles] }
        ),
      };
    };

  presentation.leafMenuGroup = /** @type { Presentation["leafMenuGroup"] } */ (
    args
  ) => {
    const link = presentation.leafMenuLink({
      text: args.text,
      link: args.link,
    });
    const groupEl = e(
      "div",
      {},
      {
        styles_: [
          contVboxStyle,
          s("leaf_menu_group_group", {
            "": (s) => {
              s.marginLeft = "0.5cm";
            },
          }),
        ],
        children_: args.children,
      }
    );
    const toggleStyle = s("leaf_menu_group_toggle", {
      "": (s) => {
        s.pointerEvents = "initial";
        s.cursor = "pointer";
        s.userSelect = "none";
        s.fontFamily = "I";
        s.fontWeight = varWIconMenuDecor;
        s.flexGrow = "0";
        s.display = "flex";
        s.justifyContent = "center";
        s.alignItems = "center";
        s.width = varSMenuIcon;
        s.height = varSMenuIcon;
        s.marginLeft = `calc(-1.0 * ${varSMenuIconShiftCalc})`;
        s.fontSize = "0.7cm";
      },
    });
    return {
      root: e(
        "details",
        {},
        {
          styles_: [
            s("leaf_menu_group_details", {
              "": (s) => {},
              [`[open]>summary>.${toggleStyle}`]: (s) => {
                s.rotate = "90deg";
              },
              ">summary::marker": (s) => {
                s.display = "none";
                s.content = '""';
              },
            }),
            topSelectableStyle,
          ],
          children_: [
            e(
              "summary",
              {},
              {
                styles_: [contHboxStyle],
                children_: [
                  e(
                    "div",
                    { textContent: textIconFoldArrow },
                    { styles_: [toggleStyle, buttonStyle] }
                  ),
                  link.root,
                ],
              }
            ),
            groupEl,
          ],
        }
      ),
      groupEl: groupEl,
      unread: link.unread,
    };
  };

  presentation.leafMenuCode = /** @type { Presentation["leafMenuCode"] } */ (
    args
  ) => {
    return { root: e("code", { textContent: args.text }, {}) };
  };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: menu, form
  const headIconButton =
    /** @type { (_:{link: string, icon: string, extraStyles?: string[]}) => HTMLElement } */ (
      args
    ) => {
      return e(
        "a",
        { href: args.link },
        {
          styles_: [buttonStyle, ...(args.extraStyles || [])],
          children_: [
            leafIcon({
              text: args.icon,
              weight: varWIconHead,
              extraStyles: [
                s("head_icon_button", {
                  "": (s) => {
                    s.color = varCForegroundVeryLight;
                    s.height = varSHeadButton;
                    s.width = varSHeadButton;
                  },
                }),
              ],
            }),
          ],
        }
      );
    };

  presentation.contNonchatHeadBar =
    /** @type { Presentation["contNonchatHeadBar"] } */ (args) => {
      const backUnread = e("div", {}, { styles_: backUnreadStyles });
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
            children_: [
              headIconButton({
                icon: textIconBack,
                link: args.backLink,
              }),
              backUnread,
            ],
          }
        )
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
          }
        )
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
            }
          )
        );
      }
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
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
          }
        ),
        backUnread: backUnread,
      };
    };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: top

  presentation.contPageTop = /** @type { Presentation["contPageTop"] } */ (
    args
  ) => {
    const appiconSize = `min(20dvw, 1.1cm)`;
    const topButton =
      /** @type { (icon: string, link: string)=>HTMLElement} */ (
        icon,
        link
      ) => {
        return e(
          "a",
          { href: link },
          {
            styles_: [buttonStyle, topSelectableStyle],
            children_: [
              createTopItemSelectBg(),
              leafIcon({
                text: icon,
                extraStyles: [
                  s("cont_page_top_generic_button", {
                    "": (s) => {
                      s.height = "100%";
                      s.opacity = "0.75";
                    },
                  }),
                ],
              }),
            ],
          }
        );
      };
    const settingsLink = topButton(textIconSettings, args.settingsLink);
    const identitiesLink = topButton(textIconIdentities, args.identitiesLink);
    const addLink = topButton(textIconAdd, args.addLink);
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
                      s.padding = "0.2cm";
                      s.gap = "0.2cm";
                      s.alignItems = "center";
                      s.justifyContent = "space-between";
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
                            s.width = appiconSize;
                            s.height = appiconSize;
                          },
                        }),
                      ],
                    }
                  ),
                  e(
                    "div",
                    {},
                    {
                      styles_: [
                        contHboxStyle,
                        s("page_top_menu_generic", {
                          "": (s) => {
                            s.height = "100%";
                            s.gap = "0.13cm";
                          },
                        }),
                      ],
                      children_: [settingsLink, identitiesLink, addLink],
                    }
                  ),
                ],
              }
            ),
            e(
              "div",
              {},
              {
                styles_: [contVboxStyle, nonchatPageStyleInner],
                children_: args.body,
              }
            ),
          ],
        }
      ),
      addLink: addLink,
      settingsLink: settingsLink,
      identitiesLink: identitiesLink,
    };
  };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: menu

  const nonchatPageStyleInner = s("nonchat_cont_page", {
    "": (s) => {
      s.margin = varPPage;
      s.marginLeft = `calc(${varPPage} + ${varSMenuIconShiftCalc})`;
      s.position = "relative";
    },
    ":before": (s) => {
      s.content = JSON.stringify("");
      s.display = "block";
      s.position = "absolute";
      s.inset = "0";
      s.backgroundColor = varCForegroundLight;
      s.maskImage = `url("linedpaper.svg")`;
      s.maskSize = `auto ${menuItemLineHeight}`;
    },
  });
  const bubbleStyle0 = s("bubble", {
    "": (s) => {
      s.borderRadius = varRBubble;
      s.backgroundColor = varCBackgroundBubble;
      //s.filter = "drop-shadow(0 0 0.08cm rgba(0,7,18, 0.35))";
      s.padding = varPBubblePadding;
    },
  });
  const bubbleStyle0Bg = s("bubble_bg", {
    "": (s) => {
      s.backgroundColor = varCBackgroundBubble;
    },
  });
  const bubbleStyles = [bubbleStyle0, bubbleStyle0Bg];
  const bubbleStylesDark = [
    bubbleStyle0,
    s("bubble_bg_dark", {
      "": (s) => {
        s.backgroundColor = varCBackgroundBubble2;
      },
    }),
  ];
  const nonchatPageStylesOuter = [
    ...bubbleStyles,
    s("cont_page_outer", {
      "": (s) => {
        s.flexGrow = "1";
        s.margin = varPPage;
      },
    }),
  ];
  presentation.contPageMenu = /** @type { Presentation["contPageMenu"] } */ (
    args
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
                  contVboxStyle,
                  nonchatPageStyleInner,
                  s("cont_page_menu_inner", {
                    "": (s) => {
                      s.width = varSPageNarrow;
                      s.alignSelf = "center";
                    },
                  }),
                ],
                children_: args.children,
              }
            ),
          ],
        }
      ),
    };
  };

  const leafIconStyle = s("icon", {
    "": (s) => {
      //s.display = "inline-grid";
      s.display = "grid";
      s.fontFamily = "I";
      s.gridTemplateColumns = "1fr";
      s.gridTemplateRows = "1fr";
      s.justifyItems = "center";
      s.alignItems = "center";
    },
    ">*": (s) => {
      s.gridColumn = "1";
      s.gridRow = "1";
    },
  });
  const leafIcon =
    /** @type {(args: {text: string, weight?: string, extraStyles?: string[]})=>HTMLElement} */ (
      args
    ) =>
      et(
        `
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
          <g transform="translate(50 50)"><text fill="currentColor" style="
            text-anchor: middle;
            dominant-baseline: central;
            font-family: I;
            font-size: 90px;
            font-weight: ${args.weight || 400};
          ">${args.text}</text></g>
        </svg>
      `,
        {
          styles_: [leafIconStyle, ...(args.extraStyles || [])],
        }
      );

  presentation.leafNonchatHeadBarCenterPlaceholder =
    /** @type { Presentation["leafNonchatHeadBarCenterPlaceholder"] } */ (
      args
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
            { styles_: [leafHeadCenterStyle] }
          ),
        };
      } else {
        return {
          root: e(
            "a",
            { textContent: args.text, href: args.link },
            {
              styles_: leafHeadCenterButtonStyles,
            }
          ),
        };
      }
    };

  presentation.leafMenuHeadBarRightAdd =
    /** @type { Presentation["leafMenuHeadBarRightAdd"] } */ (args) => {
      return { root: headIconButton({ link: args.link, icon: textIconAdd }) };
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
      }
    );
  };
  const errorStyle = s("error", {
    "": (s) => {
      s.color = varCForegroundError;
    },
  });
  presentation.contPageForm = /** @type { Presentation["contPageForm"] } */ (
    args
  ) => {
    const errors = e("div", {}, { styles_: [errorStyle] });
    const submit = e(
      "button",
      { textContent: "Ok" },
      {
        styles_: [...buttonMutStylesText],
      }
    );
    const body = e(
      "div",
      {},
      {
        styles_: [
          contVboxStyle,
          nonchatPageStyleInner,
          s("cont_page_form_inner", {
            "": (s) => {
              s.position = "relative";
            },
          }),
        ],
        children_: [...args.children],
      }
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
                  ...floatingBarStyles(varCBackgroundBubble, "0.7"),
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
              }
            ),
          ],
        }
      ),
      submit: submit,
      errors: errors,
      body: body,
    };
  };

  presentation.leafFormText = /** @type { Presentation["leafFormText"] } */ (
    args
  ) => {
    return { root: e("div", { textContent: args.text }, {}) };
  };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: chat

  presentation.contPageChat = /** @type { Presentation["contPageChat"] } */ (
    args
  ) => {
    return {
      root: e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            s("cont_page_chat", {
              "": (s) => {
                s.flexGrow = "1";
                s.position = "relative";
                s.pointerEvents = "initial";
              },
            }),
          ],
          children_: args.children,
        }
      ),
    };
  };

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
                  // Increase specificity... thanks css again
                  [`.${bubbleStyle0}`]: (s) => {
                    s.borderRadius = "999cm";
                  },
                }),
                bubbleStyle0,
              ],
              children_: [
                leafSpinner([
                  s("chat_spinner", {
                    "": (s) => {
                      s.height = "0.6cm";
                    },
                  }),
                ]),
              ],
            }
          ),
        ],
      }
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

  const floatingBarStyles =
    /** @type { (color: string, opacity: string) => string[] } */ (
      color,
      opacity
    ) => {
      return [
        s("chat_fg_style", {
          "": (s) => {
            s.backdropFilter = `blur(0.2cm)`;
            s.margin = `0.2cm 0`;
            s.padding = `0.1cm`;
          },
          ":before": (s) => {
            s.content = JSON.stringify("");
            s.display = "block";
            s.position = "absolute";
            s.inset = "0";
            s.zIndex = "-1";
          },
        }),
        s(["chat_fg_style", color, opacity], {
          "": (s) => {},
          ":before": (s) => {
            s.backgroundColor = color;
            s.opacity = opacity;
          },
        }),
      ];
    };
  const unreadStyle = s("unread", {
    "": (s) => {
      s.backgroundColor = varCNotifyBackground;
      s.backgroundBlendMode = "multiply";
      s.color = varCNotifyForeground;
      s.borderRadius = "999cm";
      s.fontFamily = "sans-serif";
      s.padding = "0.2cm";
      s.padding = "0.2cm 0.3cm";
      s.fontSize = "10pt";
      s.lineHeight = "initial";
    },
    ":empty": (s) => {
      s.display = "none";
    },
  });
  const backUnreadStyles = [unreadStyle];
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
                icon: textIconBack,
                extraStyles: [],
              }),
              backUnread,
            ],
          }
        )
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
          }
        )
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
            }
          )
        );
      }
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              ...floatingBarStyles(varCBackground, "0.7"),
              s("cont_chat_bar", {
                "": (s) => {
                  s.display = "grid";
                  s.gridTemplateColumns = "1fr auto 1fr";
                },
              }),
            ],
            children_: children,
          }
        ),
        backUnread: backUnread,
      };
    };

  const leafHeadCenterStyle = s("leaf_head_center", {
    "": (s) => {
      s.fontWeight = varWHead;
      s.fontSize = varFHeadBar;
    },
  });
  const leafHeadCenterButtonStyles = [
    buttonStyle,
    leafHeadCenterStyle,
    s("leaf_chat_center_button", {
      "": (s) => {
        s.color = varCForegroundLight;
        s.border = `${varLThin} solid ${varCForegroundVeryLight}`;
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
      args
    ) => {
      return {
        root: e(
          "span",
          { textContent: "..." },
          { styles_: leafHeadCenterButtonStyles }
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
          }
        ),
      };
    };

  // Entry
  const chatEntrySelectSpecificStyle = "chat_entry_selectable_specific";
  const chatEntrySelectableStyle = s("chat_entry_selectable", {
    "": (s) => {},
    [`.${classStateSelected} .${chatEntrySelectSpecificStyle}`]: (s) => {
      s.border = `${varLThin} solid ${varCNotifyBright}`;
    },
  });
  presentation.contChatEntryModeMessage =
    /** @type { Presentation["contChatEntryModeMessage"] } */ (args) => {
      const body = e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            chatEntrySelectSpecificStyle,
            ...(() => {
              if (args.left) {
                return bubbleStyles;
              } else {
                return bubbleStylesDark;
              }
            })(),
          ],
        }
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
              }
            ),
            body,
          ],
        }
      );
      if (args.left) {
        outer3.style.alignItems = "start";
      } else {
        outer3.style.alignItems = "end";
      }

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
              },
            }),
          ],
          children_: [
            e(
              "img",
              { src: args.image },
              {
                styles_: [
                  s("chat_portrait", {
                    "": (s) => {
                      s.borderRadius = varRPortrait;
                      s.width = varSPortrait;
                      s.height = varSPortrait;
                    },
                  }),
                ],
              }
            ),
            outer3,
          ],
        }
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
        }
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
          }
        ),
        body: body,
      };
    };
  presentation.leafChatEntryModeMessageTextBlock =
    /** @type { Presentation["leafChatEntryModeMessageTextBlock"] } */ (
      args
    ) => {
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
            children_: [e("p", { textContent: args.text }, {})],
          }
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
          }
        ),
      };
    };

  const leafChatEntryModeControlControlsButtonStyles = [
    buttonStyle,
    s("leaf_chat_entry_mode_controls_button", {
      "": (s) => {
        /*
        s.borderColor = "currentColor";
        s.borderWidth = "0.1cm";
        s.borderStyle = "solid";
        s.borderRadius = "0.1cm";
        */
        const size = "0.8cm";
        s.width = size;
        s.height = size;
        s.filter = `drop-shadow(rgba(12, 26, 47, 0.43) 0px 0px 0.05cm)`;
      },
    }),
  ];
  presentation.leafChatEntryModeControlsButtonNewMessage =
    /** @type { Presentation["leafChatEntryModeControlsButtonNewMessage"] } */ (
      args
    ) => {
      return {
        root: e(
          "button",
          {},
          {
            styles_: [
              ...leafChatEntryModeControlControlsButtonStyles,
              s("leaf_chat_entry_mode_controls_button_new_message", {
                "": (s) => {
                  s.color = "green";
                },
              }),
            ],
            children_: [
              leafIcon({
                text: textIconAddMessage,
                weight: "600",
              }),
            ],
          }
        ),
      };
    };

  // Controls
  const chatControlsBox =
    /** @type { (_:{children: Element[], extraStyles: string[]})=>HTMLElement } */ (
      args
    ) => {
      return e(
        "div",
        {},
        { styles_: [...args.extraStyles], children_: args.children }
      );
    };

  presentation.contChatControlsBarModeMenu =
    /** @type { Presentation["contChatControlsBarModeMenu"] } */ (args) => {
      return {
        root: chatControlsBox({
          children: [
            e(
              "div",
              {},
              {
                styles_: [
                  contHboxStyle,
                  ...bubbleStyles,
                  s("leaf_chat_controls_mode_menu_buttons", {
                    "": (s) => {
                      s.gap = "0.2cm";
                    },
                  }),
                ],
                children_: args.children,
              }
            ),
          ],
          extraStyles: [
            contHboxStyle,
            ...floatingBarStyles(varCBackground, "0.0"),
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
      args
    ) => {
      return {
        root: e(
          "button",
          {},
          {
            styles_: [buttonStyle],
            children_: [
              leafIcon({
                text: textIconAddMessage,
                extraStyles: [
                  s("leaf_chat_controls_bar_mode_menu_button", {
                    "": (s) => {
                      s.width = varSChatControlsButton;
                      s.height = varSChatControlsButton;
                    },
                  }),
                ],
              }),
            ],
          }
        ),
      };
    };

  presentation.leafChatControlsBarModeMessage =
    /** @type { Presentation["leafChatControlsBarModeMessage"] } */ (args) => {
      const messageButtonStyle = s("leaf_chat_controls_mode_message_button", {
        "": (s) => {
          s.width = varSChatControlsButton;
          s.height = varSChatControlsButton;
          s.filter = `drop-shadow(0 0 0.1cm ${varCBackgroundGrad2}) drop-shadow(0 0 0.1cm ${varCBackgroundGrad2})`;
        },
      });
      const close = e(
        "button",
        {},
        {
          styles_: [buttonStyle, messageButtonStyle],
          children_: [leafIcon({ text: textIconClose, weight: varWIconHead })],
        }
      );
      const send = e(
        "button",
        {},
        {
          styles_: [...buttonMutStyles, messageButtonStyle],
          children_: [leafIcon({ text: textIconSend })],
        }
      );
      const text = e(
        "div",
        { contentEditable: "plaintext-only" },
        {
          styles_: [
            s("leaf_chat_controls_mode_message_text", {
              "": (s) => {
                s.backgroundColor = "white";
                s.borderRadius = varRBubble;
                s.padding = "0.1cm 0.2cm";
                s.flexGrow = "1";
              },
            }),
          ],
        }
      );
      return {
        root: chatControlsBox({
          children: [close, text, send],
          extraStyles: [
            contHboxStyle,
            ...floatingBarStyles(varCBackground, "0.0"),
            s("leaf_chat_controls_mode_message", {
              "": (s) => {
                s.gap = "0.1cm";
              },
            }),
          ],
        }),
        send: send,
        close: close,
        text: text,
      };
    };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Assemble

  window.kwaPresentation = presentation;

  addEventListener("DOMContentLoaded", (_) => {
    const resetStyle = e(
      "link",
      { rel: "stylesheet", href: "style_reset.css" },
      {}
    );
    document.head.appendChild(resetStyle);
    notnull(document.body).classList.add(
      s("body", {
        "": (s) => {
          s.fontFamily = "X";
          //s.backgroundColor = varCBackground;
          s.background = `linear-gradient(to bottom, ${varCBackground}, ${varCBackgroundGrad2})`;
          s.color = varCForeground;
        },
      })
    );
    document.body.classList.add(contStackStyle);
  });
}
