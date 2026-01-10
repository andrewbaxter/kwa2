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
      link2.unread.textContent = "4";
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
      group1.unread.textContent = "7";
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
      group2.unread.textContent = "47";
      group2.root.classList.add(presentation.classStateSelected({}).value);
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
        presentation.classStateSelected({}).value
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
      args
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
      head.backUnread.textContent = "11";
      return presentation.contPageMenu({
        headBar: head.root,
        children: [
          presentation.leafMenuLink({ text: "Sub 1", link: "abcd" }).root,
          presentation.leafMenuButton({ text: "Sub 2" }).root,
        ],
      }).root;
    };
    const buildForm = /** @type {() => HTMLElement} */ () => {
      const page = presentation.contPageForm({
        headBar: presentation.contNonchatHeadBar({
          backLink: "abcd",
          center: presentation.leafNonchatHeadBarCenter({
            text: "Some menu",
            link: "abcd",
          }).root,
          right: undefined,
        }).root,
        children: [
          // todo
        ],
      });
      page.errors.textContent = "This is an error. Panic panic panic";
      return page.root;
    };
    const buildChat = /** @type {(_:{controls: boolean}) => HTMLElement} */ (
      args
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
      bar.style.zIndex = "1";
      if (args.controls) {
        let controls = presentation.contChatControlsBarModeMenu({
          children: [
            presentation.leafChatControlsBarModeMenuButtonNewMessage({}).root,
          ],
        });
        controls.root.style.zIndex = "1";
        topLayer.appendChild(controls.root);
      } else {
        let controls = presentation.leafChatControlsBarModeMessage({});
        controls.text.textContent = "This is a message to a distant thing.";
        controls.root.style.zIndex = "1";
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
        presentation.contChatEntryModeDeleted({ left: true }).root
      );
      const textMessage =
        /** @type {(left: Boolean,text: string)=>HTMLElement} */ (
          left,
          text
        ) => {
          const entryMessage1 = presentation.contChatEntryModeMessage({
            left: left,
            date: new Date().toISOString(),
            image: "testportrait.jpg",
          });
          entryMessage1.body.appendChild(
            presentation.leafChatEntryModeMessageTextBlock({ text: text }).root
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
        textMessage(true, "This is a short chat message")
      );
      const selectedMessage = textMessage(
        true,
        "This is a longer chat message. It contains multiple lines of text, hopefully. But it is not punishment. It is merely meant for testing. Nobody will complain if you don't read it all.\n\nI am obligated to add some new lines."
      );
      selectedMessage.classList.add(presentation.classStateSelected({}).value);
      messageLayer.appendChild(selectedMessage);

      // Right
      messageLayer.appendChild(textMessage(false, "This is a reply 1"));
      messageLayer.appendChild(
        presentation.contChatEntryModeDeleted({ left: false }).root
      );
      messageLayer.appendChild(
        textMessage(
          false,
          "This is a longer reply 1 message. It contains multiple lines of text, hopefully. But it is not punishment. It is merely meant for testing. Nobody will complain if you don't read it all.\n\nI am obligated to add some new lines."
        )
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
        presentation.leafChatEntryModeControlsButtonNewMessage({}).root
      );
      messageLayer.appendChild(controlsAsEntry);

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
      case "#form":
        {
          buildRoot(buildForm());
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
