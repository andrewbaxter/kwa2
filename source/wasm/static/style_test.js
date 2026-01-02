/// <reference path="style_export.d.ts" />
/// <reference path="style_export2.d.ts" />
{
  const presentation = window.kwaPresentation;
  addEventListener("DOMContentLoaded", async (_) => {
    const buildTop = /** @type {()=>HTMLElement} */ () => {
      return presentation.contPageTop({
        identitiesLink: "abcd",
        addLink: "abcd",
        settingsLink: "abcd",
        body: [
          presentation.leafMenuGroup({
            text: "Art",
            link: "abcd",
            children: [
              presentation.leafMenuLink({
                text: "Channel 1",
                link: "abcd",
              }).root,
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
          presentation.leafMenuGroup({
            text: "Family",
            link: "abcd",
            children: [
              presentation.leafMenuLink({
                text: "Channel 1",
                link: "abcd",
              }).root,
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
          presentation.leafMenuGroup({
            text: "Personal",
            link: "abcd",
            children: [
              presentation.leafMenuLink({
                text: "Channel 1",
                link: "abcd",
              }).root,
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
          presentation.leafMenuLink({ text: "Systems", link: "abcd" }).root,
          presentation.leafMenuLink({ text: "News", link: "abcd" }).root,
          presentation.leafAsyncBlock({}).root,
        ],
      }).root;
    };
    const buildRoot = /** @type {(wide: Boolean, e: HTMLElement)=>void} */ (
      wide,
      e
    ) => {
      if (wide) {
        document.body.appendChild(
          presentation.contRootWide({
            menu: buildTop(),
            page: e,
          }).root
        );
      } else {
        document.body.appendChild(e);
      }
    };
    const buildMenu = /** @type {() => HTMLElement} */ () => {
      return presentation.contPageMenu({
        headBar: presentation.contHeadBar({
          backLink: "abcd",
          center: presentation.leafHeadBarCenter({
            text: "Some menu",
            link: "abcd",
          }).root,
          right: undefined,
        }).root,
        children: [
          presentation.leafMenuLink({ text: "Sub 1", link: "abcd" }).root,
          presentation.leafMenuButton({ text: "Sub 2" }).root,
        ],
      }).root;
    };
    const buildForm = /** @type {() => HTMLElement} */ () => {
      return presentation.contPageForm({
        headBar: presentation.contHeadBar({
          backLink: "abcd",
          center: presentation.leafHeadBarCenter({
            text: "Some menu",
            link: "abcd",
          }).root,
          right: undefined,
        }).root,
        editBarChildren: [presentation.leafPageFormButtonSubmit({}).root],
        children: [
          // todo
        ],
      }).root;
    };
    const buildChat = /** @type {() => HTMLElement} */ () => {
      const root = document.createElement("div");
      root.style.display = "flex";
      root.style.flexDirection = "column";
      root.appendChild(
        presentation.contChatBar({
          backLink: "abcd",
          center: presentation.leafChatBarCenter({
            text: "Almonds",
            link: "abcd",
          }).root,
          right: undefined,
        }).root
      );
      root.appendChild(presentation.leafChatSpinnerEarly({}).root);
      root.appendChild(presentation.leafChatSpinnerCenter({}).root);
      root.appendChild(presentation.leafChatEntryModeDeleted({}).root);
      const entryMessage1 = presentation.leafChatEntryModeMessage({});
      entryMessage1.body.textContent = "This is a short chat message";
      root.appendChild(entryMessage1.root);
      const entryMessage2 = presentation.leafChatEntryModeMessage({});
      entryMessage2.body.textContent =
        "This is a longer chat message. It contains multiple lines of text, hopefully. But it is not punishment. It is merely meant for testing. Nobody will complain if you don't read it all.\n\nI am obligated to add some new lines.";
      root.appendChild(entryMessage2.root);
      const controlsAsEntry = presentation.contChatControlsAsEntry({}).root;
      controlsAsEntry.appendChild(
        presentation.leafChatControlsAsEntryButtonNewMessage({}).root
      );
      root.appendChild(controlsAsEntry);
      root.appendChild(presentation.leafChatSpinnerLate({}).root);
      root.appendChild(
        presentation.contChatControlsModeMenu({
          children: [
            presentation.leafChatControlsModeMenuButton({
              text: "This channel",
            }).root,
          ],
        }).root
      );
      let controlsMessage = presentation.leafChatControlsModeMessage({});
      controlsMessage.text.textContent =
        "This is a message to a distant thing.";
      root.appendChild(controlsMessage.root);
      return presentation.contPageChat({ children: [root] }).root;
    };

    const hash = location.hash;
    switch (hash) {
      case "#top":
        {
          buildRoot(false, buildTop());
        }
        break;
      case "#wide_top":
        {
          buildRoot(true, presentation.contPageBlank({}).root);
        }
        break;
      case "#menu":
        {
          buildRoot(false, buildMenu());
        }
        break;
      case "#wide_menu":
        {
          buildRoot(true, buildMenu());
        }
        break;
      case "#form":
        {
          buildRoot(false, buildForm());
        }
        break;
      case "#wide_form":
        {
          buildRoot(true, buildForm());
        }
        break;
      case "#chat":
        {
          buildRoot(false, buildChat());
        }
        break;
      case "#wide_chat":
        {
          buildRoot(true, buildChat());
        }
        break;
      default:
        throw new Error();
    }
  });
}
