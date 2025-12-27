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
    const out = /** @type {HTMLElement} */ (
      new DOMParser().parseFromString(t, "text/html").body.firstElementChild
    );
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
  const globalStyleMediaLight =
    /** @type { CSSMediaRule } */
    (
      globalStyle.cssRules[
        globalStyle.insertRule("@media (prefers-color-scheme: light) {}")
      ]
    );
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
        globalStyle.insertRule("@media not (prefers-color-scheme: light) {}")
      ]
    );
  const globalStyleDark =
    /** @type { CSSStyleRule} */
    (globalStyleMediaDark.cssRules[globalStyleMediaDark.insertRule(":root {}")])
      .style;

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

  /** @type { Set<string> } */
  const staticStyles = new Set();
  // Static style - the id must be unique for every value closed on (i.e. put all the arguments in the id).
  const s = /** @type {(
    id: string|[string],
    f: { [s: string]: (r: CSSStyleDeclaration) => void }
  ) => string} */ (id, f) => {
    const uniq = /** @type {(...args: string[]) => string} */ (...args) => {
      const lines = [];
      for (const e of notnull(new Error().stack).matchAll(/(\d+):\d+/g)) {
        lines.push(`${e[1]}`);
      }
      let uniq = [lines[1]];
      uniq.push(...args);
      return `r${uniq.join("_")}`;
    };

    let id1 = typeof id == "string" ? uniq(id) : uniq(...id);
    if (staticStyles.has(id1)) {
      return id1;
    }
    for (const [suffix, f1] of Object.entries(f)) {
      globalStyle.insertRule(`.${id1}${suffix} {}`, 0);
      f1(/** @type { CSSStyleRule } */ (globalStyle.cssRules[0]).style);
    }
    staticStyles.add(id1);
    return id1;
  };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Constants

  const textIconDelete = "\ue15b";
  const textIconAdd = "\ue145";
  const textIconSettings = "\ue8b8";
  const textIconIdentities = "\ue7fd";
  const textIconContacts = "\uf4ca";
  const textIconBack = "\ue5e0";
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
    "rgb(242, 243, 249)",
    "rgb(70, 73, 77)"
  );
  const varCForeground = vs(
    "c-foreground",
    "rgb(0, 0, 0)",
    "rgb(244, 255, 255)"
  );
  const varCForegroundError = vs(
    "c-foreground-error",
    "rgb(154, 60, 74)",
    "rgb(243, 69, 95)"
  );

  const varFMenu = "16pt";
  const varSTopButtonGeneric = "1.5cm";

  // xx State classes

  const classStateThinking = "thinking";
  presentation.classStateThinking =
    /** @type { Presentation["classStateThinking"]} */ () => ({
      value: classStateThinking,
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

  const leafSpinner = () => {
    return {
      root: e(
        "div",
        {},
        {
          styles_: [
            s("spinner", {
              "": (s) => {
                s.border = "0.06cm solid black";
                s.width = "0.5cm";
                s.height = "0.5cm";
              },
            }),
          ],
        }
      ),
    };
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
        children_: [leafSpinner().root],
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

  ///////////////////////////////////////////////////////////////////////////////
  // xx

  const leafTitleTop = /** @type { (_: {}) => {root: HTMLElement} } */ (
    args
  ) => {
    return {
      root: e(
        "div",
        {},
        { children_: [e("div", {}, {}), e("div", {}, {}), e("div", {}, {})] }
      ),
    };
  };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: root

  presentation.contRootWide = /** @type { Presentation["contRootWide"] } */ (
    args
  ) => {
    return {
      root: e(
        "div",
        {},
        {
          styles_: [
            s("cont_root_wide", {
              "": (s) => {
                s.display = "grid";
                s.gridTemplateColumns = "min(25dvw, 8cm) auto";
              },
            }),
          ],
          children_: [
            e(
              "div",
              {},
              {
                styles_: [
                  s("cont_root_wide_menu", {
                    "": (s) => {
                      s.gridColumn = "1";
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
                  s("cont_root_wide_page", {
                    "": (s) => {
                      s.gridColumn = "2";
                    },
                  }),
                ],
                children_: [args.page],
              }
            ),
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

  const menuItemStyle = s("leaf_menu_item", {
    "": (s) => {
      s.fontSize = varFMenu;
      s.display = "flex";
      s.alignItems = "center";
      s.height = "1cm";
    },
  });

  presentation.leafMenuLink = /** @type { Presentation["leafMenuLink"] } */ (
    args
  ) => {
    return {
      root: e(
        "a",
        { textContent: args.text, href: args.link },
        { styles_: [menuItemStyle] }
      ),
    };
  };

  presentation.leafMenuButton =
    /** @type { Presentation["leafMenuButton"] } */ (args) => {
      return {
        root: e(
          "button",
          { textContent: args.text },
          { styles_: [menuItemStyle] }
        ),
      };
    };

  const varMenuGroupIconSize = "1cm";
  presentation.leafMenuGroup = /** @type { Presentation["leafMenuGroup"] } */ (
    args
  ) => {
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
    return {
      root: e(
        "details",
        {},
        {
          styles_: [
            s("leaf_menu_group_details", {
              "": (s) => {},
              ">summary:before": (s) => {
                s.content = JSON.stringify(textIconFoldArrow);
                s.pointerEvents = "initial";
                s.fontFamily = "I";
                s.flexGrow = "0";
                s.display = "flex";
                s.justifyContent = "center";
                s.alignItems = "center";
                s.width = varMenuGroupIconSize;
                s.height = varMenuGroupIconSize;
                s.marginLeft = `-${varMenuGroupIconSize}`;
                s.fontSize = "0.7cm";
              },
              "[open]>summary:before": (s) => {
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
                styles_: [contHboxStyle],
                children_: [
                  presentation.leafMenuLink({
                    text: args.text,
                    link: args.link,
                  }).root,
                ],
              }
            ),
            groupEl,
          ],
        }
      ),
      groupEl: groupEl,
    };
  };

  presentation.leafMenuCode = /** @type { Presentation["leafMenuCode"] } */ (
    args
  ) => {
    return { root: e("code", { textContent: args.text }, {}) };
  };
  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: menu, form
  presentation.contHeadBar = /** @type { Presentation["contHeadBar"] } */ (
    args
  ) => {
    const children = [];
    children.push(headIconButton({ icon: textIconBack, link: args.backLink }));
    children.push(
      e(
        "div",
        {},
        {
          styles_: [
            s("cont_menu_bar_center", {
              "": (s) => {
                s.gridColumn = "2";
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
              s("cont_menu_bar_right", {
                "": (s) => {
                  s.gridColumn = "3";
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
            s("cont_menu_bar", {
              "": (s) => {
                s.display = "grid";
                s.gridTemplateColumns = "1fr auto 1fr";
              },
            }),
          ],
          children_: children,
        }
      ),
    };
  };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: top

  const paperStyle = s("paper", {
    "": (s) => {
      s.maskSize = "3cm, 3cm";
      s.maskRepeat = "repeat";
      s.maskPosition = "center";
      s.maskImage = `url("paper_mask.png")`;
      s.maskMode = "luminance";
    },
  });

  presentation.contPageTop = /** @type { Presentation["contPageTop"] } */ (
    args
  ) => {
    const appiconSize = `min(20dvw, 1.5cm)`;
    const topButton =
      /** @type { (icon: string, link: string)=>HTMLElement} */ (
        icon,
        link
      ) => {
        return e(
          "a",
          { href: link },
          {
            children_: [
              leafIcon({
                text: icon,
                extraStyles: [
                  s("cont_page_top_generic_button", {
                    "": (s) => {
                      s.height = "100%";
                    },
                  }),
                ],
              }),
            ],
          }
        );
      };
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
                            s.position = "relative";
                            s.height = `calc(${appiconSize} - 0.1cm)`;
                            s.padding = "0.13cm";
                            s.gap = "0.13cm";
                          },
                        }),
                      ],
                      children_: [
                        e(
                          "div",
                          {},
                          {
                            styles_: [
                              paperStyle,
                              s("cont_page_top_icons", {
                                "": (s) => {
                                  s.position = "absolute";
                                  s.inset = "0";
                                  s.border = `0.07cm solid ${varCForeground}`;
                                  s.borderRadius = `0.3cm`;
                                },
                              }),
                            ],
                          }
                        ),
                        topButton(textIconSettings, args.settingsLink),
                        topButton(textIconIdentities, args.identitiesLink),
                        topButton(textIconAdd, args.addLink),
                      ],
                    }
                  ),
                ],
              }
            ),
            e(
              "div",
              {},
              {
                styles_: [contVboxStyle, nonchatPageStyle],
                children_: args.body,
              }
            ),
          ],
        }
      ),
    };
  };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: menu

  const nonchatPageStyle = s("nonchat_cont_page", {
    "": (s) => {
      const padding = "0.3cm";
      s.padding = padding;
      s.paddingLeft = `calc(${padding} + ${varMenuGroupIconSize})`;
    },
  });
  presentation.contPageMenu = /** @type { Presentation["contPageMenu"] } */ (
    args
  ) => {
    return {
      root: e(
        "div",
        {},
        {
          styles_: [contVboxStyle, nonchatPageStyle],
          children_: args.children,
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
    /** @type {(args: {text: string, extraStyles?: string[]})=>HTMLElement} */ (
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
          ">${args.text}</text></g>
        </svg>
      `,
        {
          styles_: [leafIconStyle, ...(args.extraStyles || [])],
        }
      );

  const headIconButton =
    /** @type { (_:{link: string, icon: string}) => HTMLElement } */ (args) => {
      return e(
        "a",
        { href: args.link },
        { children_: [leafIcon({ text: args.icon })] }
      );
    };

  presentation.leafHeadBarCenterPlaceholder =
    /** @type { Presentation["leafHeadBarCenterPlaceholder"] } */ (args) => {
      return { root: e("span", { textContent: "..." }, {}) };
    };

  presentation.leafHeadBarCenter =
    /** @type { Presentation["leafHeadBarCenter"] } */ (args) => {
      if (args.link == null) {
        return {
          root: e("span", { textContent: args.text }, {}),
        };
      } else {
        return {
          root: e("a", { textContent: args.text, href: args.link }, {}),
        };
      }
    };

  presentation.leafMenuHeadBarRightAdd =
    /** @type { Presentation["leafMenuHeadBarRightAdd"] } */ (args) => {
      return { root: headIconButton({ link: args.link, icon: textIconAdd }) };
    };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: form

  presentation.contPageForm = /** @type { Presentation["contPageForm"] } */ (
    args
  ) => {
    return {
      root: e(
        "div",
        {},
        {
          styles_: [contVboxStyle, nonchatPageStyle],
          children_: [
            e(
              "div",
              {},
              {
                styles_: [
                  s("cont_page_form_edit_bar", {
                    "": (s) => {
                      s.position = "fixed";
                      s.left = "0";
                      s.right = "0";
                      s.bottom = "0.3cm";
                      s.height = "0.8cm";
                    },
                  }),
                ],
              }
            ),
            ...args.children,
          ],
        }
      ),
    };
  };

  presentation.contPageFormErrors =
    /** @type { Presentation["contPageFormErrors"] } */ (args) => {
      return { root: e("div", {}, {}) };
    };

  presentation.leafPageFormButtonSubmit =
    /** @type { Presentation["leafPageFormButtonSubmit"] } */ (args) => {
      return { root: e("button", { textContent: "Ok" }, {}) };
    };

  presentation.leafFormText = /** @type { Presentation["leafFormText"] } */ (
    args
  ) => {
    return { root: e("div", { textContent: args.text }, {}) };
  };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: chat

  presentation.leafChatSpinnerCenter =
    /** @type { Presentation["leafChatSpinnerCenter"] } */ (args) => {
      return {
        root: leafSpinner().root,
      };
    };

  presentation.leafChatSpinnerEarly =
    /** @type { Presentation["leafChatSpinnerCenter"] } */ (args) => {
      return {
        root: leafSpinner().root,
      };
    };

  presentation.leafChatSpinnerLate =
    /** @type { Presentation["leafChatSpinnerCenter"] } */ (args) => {
      return {
        root: leafSpinner().root,
      };
    };

  presentation.contChatBar = /** @type { Presentation["contChatBar"] } */ (
    args
  ) => {
    const children = [];
    children.push(
      e(
        "a",
        { href: args.backLink },
        {
          styles_: [
            s("cont_chat_bar_left", {
              "": (s) => {
                s.gridColumn = "1";
              },
            }),
          ],
          children_: [leafIcon({ text: textIconBack })],
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
    };
  };

  presentation.leafChatBarCenterPlaceholder =
    /** @type { Presentation["leafChatBarCenterPlaceholder"] } */ (args) => {
      return { root: e("span", { textContent: "..." }, {}) };
    };

  presentation.leafChatBarCenter =
    /** @type { Presentation["leafChatBarCenter"] } */ (args) => {
      return {
        root: e("a", { textContent: args.text, href: args.link }, {}),
      };
    };

  // Entry
  presentation.leafChatEntryModeMessage =
    /** @type { Presentation["leafChatEntryModeMessage"] } */ (args) => {
      const body = e("span", {}, {});
      return { root: e("div", {}, { children_: [body] }), body: body };
    };

  presentation.leafChatEntryModeDeleted =
    /** @type { Presentation["leafChatEntryModeDeleted"] } */ (args) => {
      return { root: e("div", {}, {}) };
    };

  presentation.contChatControlsAsEntry =
    /** @type { Presentation["contChatControlsAsEntry"] } */ (args) => {
      return { root: e("div", {}, { styles_: [contHboxStyle] }) };
    };

  presentation.leafChatControlsAsEntryButtonNewMessage =
    /** @type { Presentation["leafChatControlsAsEntryButtonNewMessage"] } */ (
      args
    ) => {
      return {
        root: e("button", {}, { children_: [leafIcon({ text: textIconAdd })] }),
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

  presentation.contChatControlsModeMenu =
    /** @type { Presentation["contChatControlsModeMenu"] } */ (args) => {
      return {
        root: chatControlsBox({ children: args.children, extraStyles: [] }),
      };
    };

  presentation.leafChatControlsModeMenuButton =
    /** @type { Presentation["leafChatControlsModeMenuButton"] } */ (args) => {
      return {
        root: e("button", {}, { children_: [leafIcon({ text: args.text })] }),
      };
    };

  presentation.leafChatControlsModeMessage =
    /** @type { Presentation["leafChatControlsModeMessage"] } */ (args) => {
      const close = e(
        "button",
        {},
        { children_: [leafIcon({ text: textIconClose })] }
      );
      const send = e(
        "button",
        {},
        { children_: [leafIcon({ text: textIconSend })] }
      );
      const text = e("div", { contentEditable: "plaintext-only" }, {});
      return {
        root: chatControlsBox({
          children: [close, send, text],
          extraStyles: [contHboxStyle],
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
    notnull(document.body.parentElement).classList.add(
      s("html", {
        "": (s) => {
          s.fontFamily = "X";
          s.backgroundColor = varCBackground;
          s.color = varCForeground;
        },
      })
    );
    document.body.classList.add(contStackStyle);
  });
}
