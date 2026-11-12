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
    const buildRoot = /** @type {( e?: HTMLElement)=>HTMLElement} */ (e) => {
      const root = presentation.contRoot({
        menu: buildTop(),
      });
      if (e != null) {
        root.page.appendChild(e);
      }
      document.body.appendChild(root.root);
      return root.page;
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
    const buildInfiniteShell =
      /** @type {(pageClass: string) => {infinite: HTMLElement, paddingPre: HTMLElement, paddingPost: HTMLElement, earlySticky: HTMLElement, real: HTMLElement, lateSpinner: HTMLElement}} */ (
        pageClass,
      ) => {
        const infinite = document.createElement("div");
        infinite.classList.add("infinite");
        infinite.classList.add(pageClass);
        infinite.style.flexGrow = "1";
        infinite.style.display = "grid";
        infinite.style.gridTemplateRows = "1fr";
        infinite.style.maxHeight = "100%";
        infinite.style.maxWidth = "100%";
        infinite.style.justifyContent = "stretch";
        infinite.style.alignItems = "stretch";

        const scrollOuter = document.createElement("div");
        scrollOuter.classList.add("scroll_outer");
        scrollOuter.style.flexGrow = "1";
        scrollOuter.style.overflowX = "hidden";
        scrollOuter.style.overflowY = "scroll";
        scrollOuter.style.scrollbarWidth = "none";
        scrollOuter.style.maxHeight = "100%";
        scrollOuter.style.pointerEvents = "initial";
        scrollOuter.style.containerType = "size";
        scrollOuter.style.isolation = "isolate";

        const stylePadding = /** @type {(e: HTMLElement) => HTMLElement} */ (
          e,
        ) => {
          e.style.display = "flex";
          e.style.flexDirection = "column";
          return e;
        };
        const paddingPre = stylePadding(document.createElement("div"));
        const paddingPost = stylePadding(document.createElement("div"));

        const scrollInner = document.createElement("div");
        scrollInner.classList.add("scroll_inner");
        scrollInner.style.display = "flex";
        scrollInner.style.flexDirection = "column";
        scrollInner.style.position = "relative";

        const contentLayout = document.createElement("div");
        contentLayout.style.position = "absolute";
        contentLayout.style.left = "0";
        contentLayout.style.right = "0";
        contentLayout.style.display = "flex";
        contentLayout.style.flexDirection = "column";

        const styleStickyHolder = /** @type {(e: HTMLElement) => HTMLElement} */ (
          e,
        ) => {
          e.classList.add("real");
          e.style.position = "sticky";
          e.style.top = "0px";
          e.style.bottom = "0px";
          e.style.zIndex = "1";
          e.style.display = "flex";
          e.style.flexDirection = "column";
          return e;
        };
        const earlySticky = styleStickyHolder(document.createElement("div"));
        const real = document.createElement("div");
        real.classList.add("real");
        real.style.display = "flex";
        real.style.flexDirection = "column";
        real.style.position = "relative";
        const lateSticky = styleStickyHolder(document.createElement("div"));

        const wrapSpinner = /** @type {(e: HTMLElement) => HTMLElement} */ (
          e,
        ) => {
          const out = document.createElement("div");
          out.appendChild(e);
          out.style.display = "flex";
          out.style.justifyContent = "center";
          out.style.alignItems = "center";
          return out;
        };
        const centerSpinner = wrapSpinner(
          presentation.leafChatSpinnerCenter({}).root,
        );
        centerSpinner.style.display = "none";
        const earlySpinner = wrapSpinner(
          presentation.leafChatSpinnerEarly({}).root,
        );
        const lateSpinner = wrapSpinner(presentation.leafChatSpinnerLate({}).root);

        scrollOuter.style.gridColumn = "1";
        scrollOuter.style.gridRow = "1";
        const overlay = document.createElement("div");
        overlay.style.display = "flex";
        overlay.style.flexDirection = "column";
        overlay.style.justifyContent = "space-between";
        overlay.style.gridColumn = "1";
        overlay.style.gridRow = "1";
        overlay.appendChild(paddingPre);
        overlay.appendChild(centerSpinner);
        overlay.appendChild(paddingPost);
        infinite.appendChild(scrollOuter);
        infinite.appendChild(overlay);
        scrollOuter.appendChild(scrollInner);
        scrollInner.appendChild(contentLayout);
        contentLayout.appendChild(earlySticky);
        contentLayout.appendChild(real);
        contentLayout.appendChild(lateSticky);
        real.appendChild(earlySpinner);

        new ResizeObserver(() => {
          scrollOuter.style.paddingTop = `${paddingPre.offsetHeight}px`;
        }).observe(paddingPre);
        new ResizeObserver(() => {
          scrollOuter.style.paddingBottom = `${paddingPost.offsetHeight}px`;
        }).observe(paddingPost);
        new ResizeObserver(() => {
          scrollInner.style.height = `${contentLayout.offsetHeight}px`;
        }).observe(contentLayout);
        return {
          infinite: infinite,
          paddingPre: paddingPre,
          paddingPost: paddingPost,
          earlySticky: earlySticky,
          real: real,
          lateSpinner: lateSpinner,
        };
      };
    const buildChat =
      /** @type {(_:{controls: boolean, stickyHolder?: boolean, blocks?: boolean, deleting?: boolean}) => HTMLElement} */ (
        args,
      ) => {
        const shell = buildInfiniteShell(presentation.classPageChat({}).value);
        const infinite = shell.infinite;
        const paddingPre = shell.paddingPre;
        const paddingPost = shell.paddingPost;
        const earlySticky = shell.earlySticky;
        const real = shell.real;
        const lateSpinner = shell.lateSpinner;

        paddingPre.appendChild(
          presentation.contChatHeadBar({
            backLink: "abcd",
            center: presentation.leafChatHeadBarCenter({
              text: "Almonds",
              link: "abcd",
            }).root,
            right: presentation.leafChatHeadBarRightCalendar({ link: "abcd" })
              .root,
          }).root,
        );
        if (args.controls) {
          paddingPost.appendChild(
            presentation.contChatControlsBarModeMenu({
              children: [
                presentation.leafChatControlsBarModeMenuButtonNewMessage({})
                  .root,
              ],
            }).root,
          );
        } else {
          const controls = args.deleting
            ? presentation.leafChatEditorModeEdit({})
            : presentation.leafChatEditorModeMessage({});
          const editor = document.createElement("div");
          editor.classList.add("malarkdowney");
          editor.contentEditable = "true";
          editor.textContent = "This is a message to a distant thing.";
          if (args.deleting) {
            const deleteButton = presentation.leafChatEditorToolboxButtonDelete({}).root;
            deleteButton.classList.add(presentation.classStateSelected({}).value);
            controls.rows.appendChild(
              presentation.contChatEditorRow({
                left: [presentation.leafChatEditorButtonClose({}).root],
                middle: presentation.contChatEditorToolbox({
                  children: [deleteButton],
                }).root,
              }).root,
            );
          } else if (args.blocks) {
            controls.rows.appendChild(
              presentation.contChatEditorRow({
                left: [presentation.leafChatEditorButtonClose({}).root],
                middle: presentation.contChatEditorToolbox({
                  children: [
                    presentation.leafChatEditorToolboxButtonCamera({}).root,
                    presentation.leafChatEditorToolboxButtonFile({}).root,
                    presentation.leafChatEditorToolboxButtonEvent({}).root,
                    presentation.leafChatEditorToolboxButtonDelete({}).root,
                  ],
                }).root,
              }).root,
            );
            controls.rows.appendChild(
              presentation.contChatEditorRow({
                left: [presentation.leafChatEditorButtonDelete({}).root],
                middle: presentation.leafChatEditorText({ editor: editor }).root,
              }).root,
            );
            for (const timeline of [
              "Tue 3 Mar 2026 9:00 - 10:00, every week on Tue, Thu, until Tue 30 Jun 2026",
              "Sat 7 Mar 2026 - Mon 9 Mar 2026",
            ]) {
              controls.rows.appendChild(
                presentation.contChatEditorRow({
                  left: [presentation.leafChatEditorButtonDelete({}).root],
                  middle: presentation.leafChatEditorEvent({ timeline: timeline }).root,
                }).root,
              );
            }
            controls.rows.appendChild(
              presentation.contChatEditorRow({
                left: [],
                middle: presentation.contChatEditorMedia({
                  children: [false, true].map(
                    (video) =>
                      presentation.leafChatEditorMediaSquare({
                        content: presentation.leafMediaThumb({
                          src: "testportrait.jpg",
                          video: video,
                          srcIsVideo: false,
                        }).root,
                      }).root,
                  ),
                }).root,
              }).root,
            );
            controls.rows.appendChild(
              presentation.contChatEditorRow({
                left: [presentation.leafChatEditorButtonDelete({}).root],
                middle: presentation.leafChatEditorFile({
                  name: "report",
                  size: "1.2 MB",
                }).root,
              }).root,
            );
          } else {
            const text = presentation.leafChatEditorText({ editor: editor });
            text.plusSlot.appendChild(
              presentation.leafChatEditorButtonPlus({}).root,
            );
            controls.rows.appendChild(
              presentation.contChatEditorRow({
                left: [presentation.leafChatEditorButtonClose({}).root],
                middle: text.root,
              }).root,
            );
          }
          paddingPost.appendChild(controls.root);
        }

        const entry = /** @type {(e: HTMLElement) => HTMLElement} */ (e) => {
          const out = presentation.contChatEntry({}).root;
          out.appendChild(e);
          return out;
        };
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
              presentation.leafChatEntryPortrait({ image: "testportrait.jpg" })
                .root,
            );
            const content = document.createElement("div");
            content.classList.add("malarkdowney");
            content.textContent = text;
            entryMessage1.body.appendChild(
              presentation.leafChatEntryMessageText({ content: content }).root,
            );
            return entryMessage1.root;
          };

        const selectedMessage = textMessage(
          true,
          "This is a longer chat message. It contains multiple lines of text, hopefully. But it is not punishment. It is merely meant for testing. Nobody will complain if you don't read it all.\n\nI am obligated to add some new lines.",
        );
        selectedMessage.classList.add(presentation.classStateSelected({}).value);
        const selectedEntry = entry(selectedMessage);
        selectedEntry.style.position = "sticky";
        selectedEntry.style.top = "0px";
        selectedEntry.style.bottom = "0px";
        selectedEntry.style.zIndex = "1";
        selectedEntry.classList.add(presentation.classStateSticky({}).value);
        if (args.stickyHolder) {
          earlySticky.appendChild(selectedEntry);
        }

        // Left
        real.appendChild(
          entry(presentation.contChatEntryModeDeleted({ left: true }).root),
        );
        for (let i = 0; i < 15; i += 1) {
          real.appendChild(entry(textMessage(true, "Spam")));
        }
        real.appendChild(entry(textMessage(true, "This is a short chat message")));
        {
          const mediaMessage = presentation.contChatEntryModeMessage({
            left: true,
            date: new Date().toISOString(),
          });
          mediaMessage.portrait.appendChild(
            presentation.leafChatEntryPortrait({ image: "testportrait.jpg" }).root,
          );
          mediaMessage.body.appendChild(
            presentation.contChatEntryMessageMedia({
              children: [
                ["testportrait.jpg", false, false],
                ["testvideo.mp4", true, true],
                ["testportrait.jpg", true, false],
                ["testportrait.jpg", false, false],
                ["testportrait.jpg", false, false],
                ["testportrait.jpg", false, false],
              ].map(
                ([src, video, srcIsVideo]) =>
                  presentation.leafChatEntryMessageMediaSquare({
                    content: presentation.leafMediaThumb({
                      src: /** @type {string} */ (src),
                      video: /** @type {boolean} */ (video),
                      srcIsVideo: /** @type {boolean} */ (srcIsVideo),
                    }).root,
                  }).root,
              ),
            }).root,
          );
          mediaMessage.body.appendChild(
            presentation.leafChatEntryMessageFile({
              name: "report.pdf",
              size: "1.2 MB",
              link: "testportrait.jpg",
            }).root,
          );
          mediaMessage.body.appendChild(
            presentation.leafChatEntryMessageExtension({
              key: "com.example.poll",
              json: JSON.stringify(
                { question: "Lunch?", options: ["Noodles", "Rice"] },
                null,
                2,
              ),
            }).root,
          );
          real.appendChild(entry(mediaMessage.root));
        }
        real.appendChild(entry(buildEventMessage(true)));
        if (!args.stickyHolder) {
          real.appendChild(selectedEntry);
        }

        // Right
        real.appendChild(entry(textMessage(false, "This is a reply 1")));
        real.appendChild(
          entry(presentation.contChatEntryModeDeleted({ left: false }).root),
        );
        real.appendChild(
          entry(
            textMessage(
              false,
              "This is a longer reply 1 message. It contains multiple lines of text, hopefully. But it is not punishment. It is merely meant for testing. Nobody will complain if you don't read it all.\n\nI am obligated to add some new lines.",
            ),
          ),
        );
        for (let i = 0; i < 9; i += 1) {
          real.appendChild(entry(textMessage(false, "Spam")));
        }

        // Controls
        const controlsAsEntry = presentation.contChatEntryModeControls({}).root;
        controlsAsEntry.appendChild(
          presentation.leafChatEntryModeControlsButtonNewMessage({}).root,
        );
        real.appendChild(controlsAsEntry);
        real.appendChild(lateSpinner);

        return infinite;
      };

    const showModal = /** @type {(e: HTMLElement) => void} */ (e) => {
      buildRoot(buildChat({ controls: false }));
      document.body.appendChild(e);
    };
    const showPageModal = /** @type {(e: HTMLElement) => void} */ (e) => {
      buildRoot(buildChat({ controls: false })).appendChild(e);
    };
    const buildMediaViewer = /** @type {(video: boolean, hidden: boolean) => HTMLElement} */ (
      video,
      hidden,
    ) => {
      const viewer = presentation.contChatModalMediaViewer({});
      if (video) {
        viewer.stage.appendChild(
          presentation.leafChatModalMediaViewerVideo({ src: "testvideo.mp4" }).root,
        );
      } else {
        viewer.stage.appendChild(
          presentation.leafChatModalMediaViewerImage({ src: "testportrait.jpg" }).root,
        );
      }
      viewer.counter.textContent = "2 / 3";
      const download = /** @type {HTMLAnchorElement} */ (viewer.download);
      download.href = video ? "testvideo.mp4" : "testportrait.jpg";
      download.download = video ? "file.mp4" : "file.jpg";
      if (hidden) {
        viewer.root.classList.add(presentation.classStateControlsHidden({}).value);
      }
      return viewer.root;
    };
    const buildCamera = /** @type {(mode: "photo"|"video"|"recording", status?: string) => HTMLElement} */ (
      mode,
      status,
    ) => {
      const camera = presentation.contChatEditorModalCamera({});
      const video = /** @type {HTMLVideoElement} */ (camera.video);
      if (status == null) {
        video.src = "testvideo.mp4";
        video.loop = true;
      } else {
        camera.status.textContent = status;
      }
      const selected = presentation.classStateSelected({}).value;
      if (mode == "photo") {
        camera.modePhoto.classList.add(selected);
      } else {
        camera.modeVideo.classList.add(selected);
        camera.shutter.classList.add(presentation.classStateVideo({}).value);
      }
      if (mode == "recording") {
        camera.shutter.classList.add(presentation.classStateRecording({}).value);
      }
      return camera.root;
    };
    const buildCameraPreview = /** @type {(video: boolean) => HTMLElement} */ (
      video,
    ) => {
      if (video) {
        return presentation.contChatEditorModalCameraPreview({
          media: presentation.leafChatEditorModalCameraPreviewVideo({ src: "testvideo.mp4" })
            .root,
          options: undefined,
        }).root;
      }
      return presentation.contChatEditorModalCameraPreview({
        media: presentation.leafChatEditorModalCameraPreviewImage({ src: "testportrait.jpg" })
          .root,
        options: presentation.leafChatEditorModalCameraPreviewSelect({
          children: [
            "Full size - 3.2 MB",
            "High quality - 1.1 MB",
            "Low quality - 240.0 KB",
          ].map((t) => {
            const o = document.createElement("option");
            o.textContent = t;
            return o;
          }),
        }).root,
      }).root;
    };

    const buildEventMessage = /** @type {(left: boolean) => HTMLElement} */ (
      left,
    ) => {
      const message = presentation.contChatEntryModeMessage({
        left: left,
        date: new Date().toISOString(),
      });
      message.portrait.appendChild(
        presentation.leafChatEntryPortrait({ image: "testportrait.jpg" }).root,
      );
      const text = document.createElement("div");
      text.classList.add("malarkdowney");
      text.textContent =
        "Standup in the big room, then the camping trip. A long description that should wrap onto several lines without breaking the layout of the message bubble.";
      message.body.appendChild(
        presentation.leafChatEntryMessageText({ content: text }).root,
      );
      for (const timeline of [
        "Tue 3 Mar 2026 9:00 - 10:00, every 2 weeks on Tue, Thu, until Tue 30 Jun 2026",
        "Sat 7 Mar 2026 - Mon 9 Mar 2026, every year",
        "Fri 13 Mar 2026 22:00 - Sat 14 Mar 2026 2:00",
      ]) {
        message.body.appendChild(
          presentation.leafChatEntryMessageEvent({ timeline: timeline }).root,
        );
      }
      return message.root;
    };

    const buildCalendarMonth =
      /** @type {(year: number, month0: number, weekStart: number, today: Date, marks: {[day: number]: [boolean, boolean]}) => HTMLElement} */ (
        year,
        month0,
        weekStart,
        today,
        marks,
      ) => {
        const days = new Date(year, month0 + 1, 0).getDate();
        const column = /** @type {(d: Date) => number} */ (d) =>
          ((((d.getDay() + 6) % 7) - weekStart + 7) % 7) + 1;
        const todayKey = new Date(
          today.getFullYear(),
          today.getMonth(),
          today.getDate(),
        ).getTime();
        const children = [];
        for (let i = 1; i <= days; i += 1) {
          const d = new Date(year, month0, i);
          const [bar, dot] = marks[i] ?? [false, false];
          children.push(
            presentation.leafCalendarDay({
              day: `${i}`,
              column: i == 1 ? `${column(d)}` : undefined,
              past: d.getTime() < todayKey,
              today: d.getTime() == todayKey,
              first: i == 1,
              last: i == days,
              bar: bar,
              dot: dot,
            }).root,
          );
        }
        const out = document.createElement("div");
        out.appendChild(
          presentation.contCalendarMonth({
            children: children,
            overlapNext: column(new Date(year, month0, days)) != 7,
          }).root,
        );
        return out;
      };

    const buildCalendar =
      /** @type {(_: {weekStart: number, head: HTMLElement}) => HTMLElement} */ (
        args,
      ) => {
        const shell = buildInfiniteShell(presentation.classPageChat({}).value);
        shell.paddingPre.appendChild(args.head);
        const today = new Date();
        for (let offset = -3; offset <= 4; offset += 1) {
          const first = new Date(today.getFullYear(), today.getMonth() + offset, 1);
          /** @type {{[day: number]: [boolean, boolean]}} */
          const marks = {};
          const seed = first.getFullYear() * 12 + first.getMonth();
          for (let i = 0; i < 6; i += 1) {
            const day = ((seed * 7 + i * 11) % 28) + 1;
            marks[day] = [i % 3 == 0, i % 2 == 1];
          }
          for (let day = 10; day <= 13; day += 1) {
            marks[day] = [true, (marks[day] ?? [false, false])[1]];
          }
          shell.real.appendChild(
            buildCalendarMonth(
              first.getFullYear(),
              first.getMonth(),
              args.weekStart,
              today,
              marks,
            ),
          );
        }
        shell.real.appendChild(shell.lateSpinner);
        return shell.infinite;
      };

    const buildCalendarChannelHead = () =>
      presentation.contChatHeadBar({
        backLink: "abcd",
        center: presentation.leafChatHeadBarCenter({
          text: "Almonds",
          link: "abcd",
        }).root,
        right: presentation.leafChatHeadBarRightMessages({ link: "abcd" }).root,
      }).root;

    const buildCalendarStack = /** @type {(layers: HTMLElement[]) => HTMLElement} */ (
      layers,
    ) => {
      const stack = presentation.contCalendarStack({}).root;
      for (const layer of layers) {
        stack.appendChild(layer);
      }
      return stack;
    };

    const buildCalendarModal = /** @type {(layer: HTMLElement) => HTMLElement} */ (
      layer,
    ) => {
      const modal = presentation.contCalendarModal({}).root;
      modal.appendChild(layer);
      return modal;
    };

    const buildModalHead = /** @type {(title: string, right?: HTMLElement) => HTMLElement} */ (
      title,
      right,
    ) =>
      presentation.contNonchatHeadBar({
        backLink: "abcd",
        center: presentation.leafNonchatHeadBarCenter({ text: title, link: undefined })
          .root,
        right: right,
      }).root;

    const buildCalendarDay =
      /** @type {(_: {select: boolean, empty: boolean}) => HTMLElement} */ (
        args,
      ) => {
        const children = [];
        if (args.empty) {
          children.push(presentation.leafCalendarDayEmpty({}).root);
        } else {
          children.push(buildEventMessage(true));
          const selected = buildEventMessage(false);
          selected.classList.add(presentation.classStateSelected({}).value);
          children.push(selected);
          children.push(buildEventMessage(true));
        }
        return presentation.contCalendarPage({
          headBar: buildModalHead("Tuesday 10 March 2026"),
          body: presentation.contCalendarDayList({ children: children }).root,
          controls: presentation.contChatControlsBarModeMenu({
            children: [
              args.select
                ? presentation.leafCalendarDayButtonSelect({}).root
                : presentation.leafCalendarDayButtonGoTo({}).root,
            ],
          }).root,
        }).root;
      };

    const buildCalendarChannels = () =>
      presentation.contPageMenu({
        headBar: buildModalHead("Select calendar"),
        children: [
          "Almonds",
          "Family",
          "A channel with a very long name that should be cut off somewhere",
        ].map((t) => presentation.leafMenuButton({ text: t }).root),
      }).root;

    const buildEventEditor =
      /** @type {(_: {allDay: boolean, repeat: "none"|"weeks"|"months", ends: "never"|"until"|"count"}) => HTMLElement} */ (
        args,
      ) => {
        const date = /** @type {(label: string, value: string) => HTMLElement} */ (
          label,
          value,
        ) =>
          rf.el("input", { "aria-label": label, type: "text", readonly: "", value: value }, [
            "rf_input_small",
          ]);
        const time = /** @type {(label: string, value: string) => HTMLElement} */ (
          label,
          value,
        ) =>
          rf.el("input", { "aria-label": label, type: "time", value: value }, [
            "rf_input_small",
          ]);
        const rangeFields = args.allDay
          ? [
              rf.label("Start"),
              date("Start", "Tue, Mar 3, 2026"),
              rf.label("End"),
              date("End", "Thu, Mar 5, 2026"),
            ]
          : [
              rf.label("Start date"),
              date("Start date", "Tue, Mar 3, 2026"),
              rf.error(),
              rf.label("Start time"),
              time("Start time", "09:00"),
              rf.label("End date"),
              date("End date", "Tue, Mar 3, 2026"),
              rf.error(),
              rf.label("End time"),
              time("End time", "10:00"),
            ];
        const children = [
          rf.label("When"),
          rf.select("When", ["At a time", "All day"], args.allDay ? 1 : 0),
          rf.subform(1, rangeFields),
          rf.label("Repeat"),
          rf.select(
            "Repeat",
            ["Doesn't repeat", "Daily", "Weekly", "Monthly", "Yearly"],
            { none: 0, weeks: 2, months: 3 }[args.repeat],
          ),
        ];
        if (args.repeat == "none") {
          children.push(rf.subform(1, []));
        } else {
          const endSub = {
            never: [],
            until: [date("On date", "Tue, Jun 30, 2026")],
            count: [rf.error(), rf.text("After a number of times", "10")],
          }[args.ends];
          const repeatFields = [
            rf.error(),
            rf.label("Every"),
            rf.text("Every", "2"),
            rf.label("On"),
          ];
          if (args.repeat == "weeks") {
            repeatFields.push(
              rf.el(
                "div",
                {},
                ["rf_input_small"],
                ["M", "T", "W", "T", "F", "S", "S"].map((d, i) => {
                  const box = rf.checkbox(d, i == 1 || i == 3);
                  box.classList.remove("rf_input_small");
                  const label = rf.el("label", {}, [], [box, rf.el("span", {}, [])]);
                  /** @type {HTMLElement} */ (label.lastChild).textContent = d;
                  return label;
                }),
              ),
            );
          } else {
            repeatFields.push(
              rf.select(
                "On",
                [
                  "Same day of the month",
                  "Same weekday and week of the month",
                  "Last of that weekday in the month",
                ],
                1,
              ),
              rf.subform(2, []),
            );
          }
          repeatFields.push(
            rf.label("Ends"),
            rf.select(
              "Ends",
              ["Never", "On date", "After a number of times"],
              { never: 0, until: 1, count: 2 }[args.ends],
            ),
            rf.subform(2, endSub),
          );
          children.push(rf.subform(1, repeatFields));
        }
        return buildFormPage("Event", children);
      };

    /** @type { {[hash: string]: () => Promise<void>} } */
    const tests = {
      "#top": async () => {
        buildRoot();
      },
      "#menu": async () => {
        buildRoot(buildMenu({ link: true }));
      },
      "#menu_nolink": async () => {
        buildRoot(buildMenu({ link: false }));
      },
      "#menu_invite": async () => {
        buildRoot(await buildMenuInvite());
      },
      "#form": async () => {
        buildRoot(buildForm());
      },
      "#form_confirm": async () => {
        buildRoot(buildFormConfirm());
      },
      "#form_all": async () => {
        buildRoot(buildFormAll());
      },
      "#chat": async () => {
        buildRoot(buildChat({ controls: false }));
      },
      "#chat_sticky_holder": async () => {
        buildRoot(buildChat({ controls: false, stickyHolder: true }));
      },
      "#chat_editor_blocks": async () => {
        buildRoot(buildChat({ controls: false, blocks: true }));
      },
      "#chat_editor_deleting": async () => {
        buildRoot(buildChat({ controls: false, deleting: true }));
      },
      "#chat_controls_menu": async () => {
        buildRoot(buildChat({ controls: true }));
      },
      "#chat_editor_modal_confirm": async () => {
        showModal(presentation.contChatEditorModalConfirm({ text: "Delete the text?" }).root);
      },
      "#chat_modal_viewer_image": async () => {
        showModal(buildMediaViewer(false, false));
      },
      "#chat_modal_viewer_image_hidden": async () => {
        showModal(buildMediaViewer(false, true));
      },
      "#chat_modal_viewer_video": async () => {
        showModal(buildMediaViewer(true, false));
      },
      "#chat_editor_modal_camera_photo": async () => {
        showModal(buildCamera("photo"));
      },
      "#chat_editor_modal_camera_video": async () => {
        showModal(buildCamera("video"));
      },
      "#chat_editor_modal_camera_recording": async () => {
        showModal(buildCamera("recording"));
      },
      "#chat_editor_modal_camera_error": async () => {
        showModal(
          buildCamera("photo", "Error requesting camera: NotAllowedError"),
        );
      },
      "#chat_editor_modal_camera_preview_photo": async () => {
        showModal(buildCameraPreview(false));
      },
      "#chat_editor_modal_camera_preview_video": async () => {
        showModal(buildCameraPreview(true));
      },
      "#calendar": async () => {
        buildRoot(
          buildCalendarStack([
            buildCalendar({ weekStart: 0, head: buildCalendarChannelHead() }),
          ]),
        );
      },
      "#calendar_week_sunday": async () => {
        buildRoot(
          buildCalendarStack([
            buildCalendar({ weekStart: 6, head: buildCalendarChannelHead() }),
          ]),
        );
      },
      "#calendar_day": async () => {
        buildRoot(
          buildCalendarStack([
            buildCalendar({ weekStart: 0, head: buildCalendarChannelHead() }),
            buildCalendarDay({ select: false, empty: false }),
          ]),
        );
      },
      "#calendar_day_empty": async () => {
        buildRoot(
          buildCalendarStack([
            buildCalendar({ weekStart: 0, head: buildCalendarChannelHead() }),
            buildCalendarDay({ select: false, empty: true }),
          ]),
        );
      },
      "#event_editor_timed_weekly": async () => {
        showPageModal(buildCalendarModal(buildEventEditor({ allDay: false, repeat: "weeks", ends: "until" })));
      },
      "#event_editor_all_day_monthly": async () => {
        showPageModal(buildCalendarModal(buildEventEditor({ allDay: true, repeat: "months", ends: "count" })));
      },
      "#event_editor_no_repeat": async () => {
        showPageModal(buildCalendarModal(buildEventEditor({ allDay: false, repeat: "none", ends: "never" })));
      },
      "#calendar_picker": async () => {
        showPageModal(
          buildCalendarModal(
            presentation.contCalendarPage({
              headBar: buildModalHead(
                "Almonds",
                presentation.leafCalendarHeadBarButtonSelectCalendar({}).root,
              ),
              body: buildCalendar({ weekStart: 0, head: document.createElement("div") }),
              controls: undefined,
            }).root,
          ),
        );
      },
      "#calendar_picker_day": async () => {
        showPageModal(buildCalendarModal(buildCalendarDay({ select: true, empty: false })));
      },
      "#calendar_picker_channels": async () => {
        showPageModal(buildCalendarModal(buildCalendarChannels()));
      },
    };
    addEventListener("hashchange", () => location.reload());
    const test = tests[location.hash];
    if (test != null) {
      await test();
    } else {
      const index = document.createElement("div");
      index.style.display = "flex";
      index.style.flexDirection = "column";
      index.style.gap = "0.2cm";
      index.style.padding = "0.5cm";
      for (const hash of Object.keys(tests)) {
        const link = document.createElement("a");
        link.href = hash;
        link.textContent = hash;
        link.style.textDecoration = "underline";
        index.appendChild(link);
      }
      document.body.appendChild(index);
    }
  });
}
