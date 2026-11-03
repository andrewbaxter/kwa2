/// <reference path="style_export.d.ts" />
/// <reference path="style_export2.d.ts" />
{
  const presentation = window.kwaPresentation;
  addEventListener("DOMContentLoaded", async (_) => {
    const buildTop = /** @type {()=>HTMLElement} */ () => {
      const link1 = presentation.leafMenuLink({
        text: "Channel 1",
        link: "abcd",
        image: "testportrait.jpg",
      });
      link1.root.classList.add(presentation.classStateSelected({}).value);
      const link2 = presentation.leafMenuLink({
        text: "Channel 1",
        link: "abcd",
      });
      link2.unread.classList.remove(presentation.classStateHidden({}).value);
      const group1 = presentation.leafMenuGroup({
        text: "Family",
        link: "abcd",
        children: [
          link1.root,
          presentation.leafMenuLink({
            text: "Channel 2",
            link: "abcd",
          }).root,
          presentation.leafMenuLink({
            text: "Channel 3",
            link: "abcd",
          }).root,
        ],
      });
      group1.unread.classList.remove(presentation.classStateHidden({}).value);
      const group2 = presentation.leafMenuGroup({
        text: "Personal this is a long group",
        link: "abcd",
        children: [
          presentation.leafMenuLink({
            text: "Channel 1",
            link: "abcd",
            image: "testportrait.jpg",
          }).root,
          presentation.leafMenuLink({
            text: "Channel 2",
            link: "abcd",
            image: "testportrait.jpg",
          }).root,
          presentation.leafMenuLink({
            text: "Channel 3",
            link: "abcd",
          }).root,
        ],
      });
      group2.unread.classList.remove(presentation.classStateHidden({}).value);
      group2.link.classList.add(presentation.classStateSelected({}).value);
      const page = presentation.contPageTop({
        identitiesLink: "abcd",
        addLink: "abcd",
        settingsLink: "abcd",
        body: [
          presentation.leafMenuGroup({
            text: "Art",
            link: "abcd",
            children: [
              link2.root,
              presentation.leafMenuLink({
                text: "Channel 2",
                link: "abcd",
              }).root,
              presentation.leafMenuLink({
                text: "Channel 3",
                link: "abcd",
              }).root,
            ],
          }).root,
          group1.root,
          group2.root,
          presentation.leafMenuLink({
            text: "Systems",
            link: "abcd",

            image: "testportrait.jpg",
          }).root,
          presentation.leafMenuLink({
            text: "News",
            link: "abcd",

            image: "testportrait.jpg",
          }).root,
          presentation.leafAsyncBlock({}).root,
        ],
      });
      page.settingsLink.classList.add(
        presentation.classStateSelected({}).value,
      );
      return page.root;
    };
    const buildRoot = /** @type {( e?: HTMLElement)=>void} */ (e) => {
      const root = presentation.contRoot({
        menu: buildTop(),
      });
      if (e != null) {
        root.page.appendChild(e);
      }
      document.body.appendChild(root.root);
    };
    const buildMenu = /** @type {(_:{link: Boolean}) => HTMLElement} */ (
      args,
    ) => {
      /** @type { undefined|string } */
      let centerLink;
      if (args.link) {
        centerLink = "abcd";
      } else {
        centerLink = undefined;
      }
      const head = presentation.contNonchatHeadBar({
        backLink: "abcd",
        center: presentation.leafNonchatHeadBarCenter({
          text: "Some menu",
          link: centerLink,
        }).root,
        right: undefined,
      });
      head.backUnread.classList.remove(presentation.classStateHidden({}).value);
      return presentation.contPageMenu({
        headBar: head.root,
        children: [
          presentation.leafMenuLink({ text: "Sub 1", link: "abcd" }).root,
          presentation.leafMenuButton({ text: "Sub 2" }).root,
        ],
      }).root;
    };
    const buildMenuInvite = /** @type {() => Promise<HTMLElement>} */ async () => {
      const head = presentation.contNonchatHeadBar({
        backLink: "abcd",
        center: presentation.leafNonchatHeadBarCenter({
          text: "Invite for Bob",
          link: undefined,
        }).root,
        right: undefined,
      });
      return presentation.contPageMenu({
        headBar: head.root,
        children: [
          presentation.leafMenuQr({
            svg: await (await fetch("testqr.svg")).text(),
          }).root,
          presentation.leafMenuCode({
            text: "kwa:in/c?identity=yryyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy&channel=12345&code=XYZXYZXYZXYZ",
          }).root,
          presentation.leafMenuLink({ text: "Edit", link: "abcd" }).root,
          presentation.leafMenuLink({ text: "Delete", link: "abcd" }).root,
        ],
      }).root;
    };
    // Mirrors the DOM rooting_forms generates (rf_* classes) inside a form page.
    /** @type {(tag: string, attrs: Record<string,string>, classes: string[], children?: HTMLElement[]) => HTMLElement} */
    const rfEl = (tag, attrs, classes, children) => {
      const out = document.createElement(tag);
      for (const [k, v] of Object.entries(attrs)) {
        out.setAttribute(k, v);
      }
      out.classList.add(...classes);
      for (const c of children ?? []) {
        out.appendChild(c);
      }
      return out;
    };
    /** @type {(text: string) => HTMLElement} */
    const rfLabel = (text) => {
      const out = rfEl("span", {}, ["rf_label"]);
      out.textContent = text;
      return out;
    };
    /** @type {(text?: string) => HTMLElement} */
    const rfError = (text) => {
      const out = rfEl("div", {}, ["rf_error"]);
      out.textContent = text ?? "";
      return out;
    };
    /** @type {(depth: number, children: HTMLElement[]) => HTMLElement} */
    const rfSubform = (depth, children) =>
      rfEl("div", {}, ["rf_subform", `rf_depth_m7_${1 + (depth % 7)}`], children);
    const rf = {
      el: rfEl,
      label: rfLabel,
      error: rfError,
      /** @type {(label: string, value: string) => HTMLElement} */
      text: (label, value) => {
        const out = rfEl(
          "div",
          { "aria-label": label, contenteditable: "plaintext-only" },
          ["rf_input_small", "rf_input_text"],
        );
        out.textContent = value;
        return out;
      },
      /** @type {(label: string, value: string) => HTMLElement} */
      bigText: (label, value) => {
        const out = rfEl(
          "div",
          { "aria-label": label, contenteditable: "plaintext-only" },
          ["rf_input_big", "rf_input_text"],
        );
        out.textContent = value;
        return out;
      },
      /** @type {(label: string, value: string) => HTMLElement} */
      password: (label, value) =>
        rfEl("input", { "aria-label": label, type: "password", value: value }, [
          "rf_input_small",
        ]),
      /** @type {(label: string, checked: boolean) => HTMLElement} */
      checkbox: (label, checked) => {
        const out = rfEl("input", { "aria-label": label, type: "checkbox" }, [
          "rf_input_small",
        ]);
        if (checked) {
          out.setAttribute("checked", "checked");
        }
        return out;
      },
      /** @type {(label: string, options: string[], selected: number) => HTMLElement} */
      select: (label, options, selected) => {
        const out = rfEl("select", { "aria-label": label }, ["rf_input_small"]);
        options.forEach((o, i) => {
          const opt = rfEl("option", { value: `${i}` }, []);
          opt.textContent = o;
          if (i === selected) {
            opt.setAttribute("selected", "selected");
          }
          out.appendChild(opt);
        });
        return out;
      },
      /** @type {(label: string, previewSrc?: string) => HTMLElement} */
      file: (label, previewSrc) => {
        const remove = rfEl(
          "button",
          { type: "button", "aria-label": `${label} - Remove`, title: `${label} - Remove` },
          ["rf_button_icon", "rf_button_delete"],
        );
        if (previewSrc == null) {
          remove.classList.add("disable_hide");
        }
        return rfEl("div", {}, ["rf_file"], [
          rfEl(
            "div",
            {},
            ["rf_file_preview"],
            previewSrc == null ? [] : [rfEl("img", { src: previewSrc }, [])],
          ),
          rfEl("input", { "aria-label": label, type: "file", accept: "image/*" }, [
            "rf_input_small",
          ]),
          remove,
        ]);
      },
      subform: rfSubform,
      /** @type {(field: string, label: string, el: HTMLElement) => HTMLElement[]} */
      textField: (field, label, el) => [rfError(), rfLabel(label), el],
      /** @type {(label: string, items: HTMLElement[][], depth: number) => HTMLElement} */
      vec: (label, items, depth) => {
        const addButton = () =>
          rfEl(
            "button",
            { "aria-label": `${label} - Add new item`, title: `${label} - Add new item` },
            ["rf_button_icon", "rf_button_add"],
          );
        const itemsEl = rfEl(
          "div",
          {},
          ["rf_vec_items"],
          items.map((fields) =>
            rfEl("div", {}, [], [
              addButton(),
              rfSubform(depth, [
                rfEl("div", {}, ["rf_vec_item_header"], [
                  rfEl("span", {}, []),
                  rfEl("button", { "aria-label": `${label} - Move item up` }, [
                    "rf_button_icon",
                    "rf_button_move_up",
                  ]),
                  rfEl("button", { "aria-label": `${label} - Move item down` }, [
                    "rf_button_icon",
                    "rf_button_move_down",
                  ]),
                  rfEl("button", { "aria-label": `${label} - Delete item` }, [
                    "rf_button_icon",
                    "rf_button_delete",
                  ]),
                ]),
                ...fields,
              ]),
            ]),
          ),
        );
        return rfEl("div", {}, ["rf_vec"], [itemsEl, addButton()]);
      },
    };
    /** @type {(title: string, formChildren: HTMLElement[], error?: string) => HTMLElement} */
    const buildFormPage = (title, formChildren, error) => {
      const page = presentation.contPageForm({
        headBar: presentation.contNonchatHeadBar({
          backLink: "abcd",
          center: presentation.leafNonchatHeadBarCenter({
            text: title,
            link: undefined,
          }).root,
          right: undefined,
        }).root,
        children: [],
      });
      page.body.appendChild(rf.el("form", {}, [], formChildren));
      if (error != null) {
        page.errors.appendChild(
          presentation.leafErrBlock({ data: error }).root,
        );
      }
      return page.root;
    };
    const buildForm = /** @type {() => HTMLElement} */ () => {
      const url = rf.text(
        "URL",
        "kwa:in/c/abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789/12345/XYZXYZ",
      );
      url.classList.add(presentation.classCode({}).value);
      return buildFormPage("Join by URL", [
        rfError(),
        rfLabel("URL"),
        url,
      ]);
    };
    const buildFormConfirm = /** @type {() => HTMLElement} */ () => {
      return buildFormPage("Join by URL", [
        rfError(),
        rfLabel("Join as"),
        rf.select("Join as", ["Personal", "Work"], 0),
        rfError(),
        rfLabel("Name"),
        rf.text("Name", "Almonds"),
        rfError(),
        rfLabel("Description"),
        rf.text("Description", "A channel about almonds and almond adjacent nuts."),
        rfError(),
        rfLabel("Portrait"),
        rf.file("Portrait", "testportrait.jpg"),
      ]);
    };
    // Every element kind rooting_forms can produce, in one page.
    const buildFormAll = /** @type {() => HTMLElement} */ () => {
      return buildFormPage(
        "All form elements",
        [
          rfError(),
          rfLabel("Text"),
          rf.text("Text", "Some text"),
          rfError("This field has a validation error"),
          rfLabel("Text with error"),
          rf.text("Text with error", "not a number"),
          rfLabel("Big text"),
          rf.bigText(
            "Big text",
            "A longer block of text.\nIt spans multiple lines and both columns.",
          ),
          rfLabel("Password"),
          rf.password("Password", "hunter2"),
          rfLabel("Checkbox"),
          rf.checkbox("Checkbox", true),
          rfLabel("Optional (enabled)"),
          rf.el("input", { "aria-label": "Optional (enabled) - Enabled", type: "checkbox", checked: "checked" }, [
            "rf_input_option",
          ]),
          rfError(),
          rf.text("Optional (enabled)", "Present"),
          rfLabel("Optional (disabled)"),
          rf.el("input", { "aria-label": "Optional (disabled) - Enabled", type: "checkbox" }, [
            "rf_input_option",
          ]),
          rf.el("div", {}, ["rf_error", "disable_hide"]),
          rf.el(
            "div",
            { "aria-label": "Optional (disabled)", contenteditable: "plaintext-only" },
            ["rf_input_small", "rf_input_text", "disable_hide"],
          ),
          rfLabel("Enum with subform"),
          rf.select("Enum with subform", ["Unit variant", "Struct variant"], 1),
          rfSubform(0, [
            rfError(),
            rfLabel("Inner text"),
            rf.text("Inner text", "Inside the variant"),
            rfLabel("Inner checkbox"),
            rf.checkbox("Inner checkbox", false),
          ]),
          rfError(),
          rfLabel("Portrait"),
          rf.file("Portrait", "testportrait.jpg"),
          rfLabel("File (empty)"),
          rf.file("File (empty)"),
          rfLabel("List"),
          rf.vec(
            "List",
            [
              [
                rfError(),
                rfLabel("Item text"),
                rf.text("Item text", "First"),
                rfLabel("Item kind"),
                rf.select("Item kind", ["Unit variant", "Struct variant"], 0),
                rfSubform(1, []),
              ],
              [
                rfError(),
                rfLabel("Item text"),
                rf.text("Item text", "Second"),
                rfLabel("Item kind"),
                rf.select("Item kind", ["Unit variant", "Struct variant"], 1),
                rfSubform(1, [
                  rfError(),
                  rfLabel("Inner text"),
                  rf.text("Inner text", "Nested two deep"),
                ]),
              ],
            ],
            0,
          ),
          presentation.leafFormText({ text: "Are you sure you want to delete this thing?" }).root,
          presentation.leafAsyncBlock({}).root,
        ],
        "This is an error. Panic panic panic",
      );
    };
    const buildChat = /** @type {(_:{controls: boolean}) => HTMLElement} */ (
      args,
    ) => {
      const topLayer = document.createElement("div");
      topLayer.style.display = "flex";
      topLayer.style.flexDirection = "column";
      topLayer.style.justifyContent = "space-between";
      const bar = presentation.contChatHeadBar({
        backLink: "abcd",
        center: presentation.leafChatHeadBarCenter({
          text: "Almonds",
          link: "abcd",
        }).root,
        right: undefined,
      }).root;
      topLayer.appendChild(bar);
      if (args.controls) {
        let controls = presentation.contChatControlsBarModeMenu({
          children: [
            presentation.leafChatControlsBarModeMenuButtonNewMessage({}).root,
          ],
        });
        topLayer.appendChild(controls.root);
      } else {
        let controls = presentation.leafChatControlsBarModeMessage({});
        controls.text.textContent = "This is a message to a distant thing.";
        topLayer.appendChild(controls.root);
      }

      const messageLayer = document.createElement("div");
      messageLayer.style.display = "flex";
      messageLayer.style.flexDirection = "column";
      messageLayer.style.overflowY = "scroll";
      messageLayer.style.height = "100dvh";
      messageLayer.style.pointerEvents = "initial";
      messageLayer.style.padding = ``;
      messageLayer.appendChild(presentation.leafChatSpinnerEarly({}).root);
      messageLayer.appendChild(presentation.leafChatSpinnerCenter({}).root);
      messageLayer.appendChild(
        presentation.contChatEntryModeDeleted({ left: true }).root,
      );
      const textMessage =
        /** @type {(left: Boolean,text: string)=>HTMLElement} */ (
          left,
          text,
        ) => {
          const entryMessage1 = presentation.contChatEntryModeMessage({
            left: left,
            date: new Date().toISOString(),
          });
          entryMessage1.portrait.appendChild(
            presentation.leafChatEntryPortrait({ image: "testportrait.jpg" }).root,
          );
          entryMessage1.body.appendChild(
            presentation.leafChatEntryModeMessageTextBlock({ text: text }).root,
          );
          return entryMessage1.root;
        };

      // Left
      messageLayer.appendChild(textMessage(true, "Spam"));
      messageLayer.appendChild(textMessage(true, "Spam"));
      messageLayer.appendChild(textMessage(true, "Spam"));
      messageLayer.appendChild(textMessage(true, "Spam"));
      messageLayer.appendChild(textMessage(true, "Spam"));
      messageLayer.appendChild(
        textMessage(true, "This is a short chat message"),
      );
      const selectedMessage = textMessage(
        true,
        "This is a longer chat message. It contains multiple lines of text, hopefully. But it is not punishment. It is merely meant for testing. Nobody will complain if you don't read it all.\n\nI am obligated to add some new lines.",
      );
      selectedMessage.classList.add(presentation.classStateSelected({}).value);
      messageLayer.appendChild(selectedMessage);

      // Right
      messageLayer.appendChild(textMessage(false, "This is a reply 1"));
      messageLayer.appendChild(
        presentation.contChatEntryModeDeleted({ left: false }).root,
      );
      messageLayer.appendChild(
        textMessage(
          false,
          "This is a longer reply 1 message. It contains multiple lines of text, hopefully. But it is not punishment. It is merely meant for testing. Nobody will complain if you don't read it all.\n\nI am obligated to add some new lines.",
        ),
      );
      messageLayer.appendChild(textMessage(false, "Spam"));
      messageLayer.appendChild(textMessage(false, "Spam"));
      messageLayer.appendChild(textMessage(false, "Spam"));
      messageLayer.appendChild(textMessage(false, "Spam"));
      messageLayer.appendChild(textMessage(false, "Spam"));
      messageLayer.appendChild(textMessage(false, "Spam"));
      messageLayer.appendChild(textMessage(false, "Spam"));
      messageLayer.appendChild(textMessage(false, "Spam"));
      messageLayer.appendChild(textMessage(false, "Spam"));
      messageLayer.appendChild(presentation.leafChatSpinnerLate({}).root);

      // Controls
      const controlsAsEntry = presentation.contChatEntryModeControls({}).root;
      controlsAsEntry.appendChild(
        presentation.leafChatEntryModeControlsButtonNewMessage({}).root,
      );
      messageLayer.appendChild(controlsAsEntry);

      const padding = document.createElement("div");
      padding.style.height = "1cm";
      messageLayer.appendChild(padding);

      const root = document.createElement("div");
      root.classList.add("stack");
      root.appendChild(messageLayer);
      root.appendChild(topLayer);

      return presentation.contPageChat({ children: [root] }).root;
    };

    const hash = location.hash;
    switch (hash) {
      case "#top":
        {
          buildRoot();
        }
        break;
      case "#menu":
        {
          buildRoot(buildMenu({ link: true }));
        }
        break;
      case "#menu_nolink":
        {
          buildRoot(buildMenu({ link: false }));
        }
        break;
      case "#menu_invite":
        {
          buildRoot(await buildMenuInvite());
        }
        break;
      case "#form":
        {
          buildRoot(buildForm());
        }
        break;
      case "#form_confirm":
        {
          buildRoot(buildFormConfirm());
        }
        break;
      case "#form_all":
        {
          buildRoot(buildFormAll());
        }
        break;
      case "#chat":
        {
          buildRoot(buildChat({ controls: false }));
        }
        break;
      case "#chat_controls":
        {
          buildRoot(buildChat({ controls: true }));
        }
        break;
      default:
        throw new Error();
    }
  });
}
