/// <reference path="style_export.d.ts" />
/// <reference path="style_export2.d.ts" />
{
  const presentation = window.sunwetPresentation;
  addEventListener("DOMContentLoaded", async (_) => {
    const buildRoot = /** @type {(e: HTMLElement[])=>void} */ (e) => {
      document.body.appendChild(
        presentation.contRootStack({ children: e }).root
      );
    };

    const hash = location.hash;
    switch (hash) {
      case "#account":
        {
          buildRoot([]);
        }
        break;
      case "#identities":
        {
          buildRoot([]);
        }
        break;
      case "#channels":
        {
          buildRoot([
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "" }).root,
              mainBody: presentation.contPageHome({}).root,
              menuBody: stagingMenu,
            }).root,
          ]);
        }
        break;
      case "#logs":
        {
          buildRoot([
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "Logs" }).root,
              mainBody: presentation.contPageLogs({
                children: [
                  presentation.leafLogsLine({
                    stamp: new Date().toISOString(),
                    text: "short line",
                  }).root,
                  presentation.leafLogsLine({
                    stamp: new Date().toISOString(),
                    text: "very log line with lots of log in it this may wrap or it might not we'll have to see in practice but it's already wrapping in my editor",
                  }).root,
                  presentation.leafLogsLine({
                    stamp: new Date().toISOString(),
                    text: "short line",
                  }).root,
                  presentation.leafLogsLine({
                    stamp: new Date().toISOString(),
                    text: "short line",
                  }).root,
                ],
              }).root,
              menuBody: stagingMenu,
            }).root,
          ]);
        }
        break;
      case "#view":
        {
          buildRoot([stagingPageView]);
        }
        break;
      case "#menu":
        {
          buildRoot([stagingPageView]);
          for (const e of document.getElementsByClassName(
            presentation.classMenuWantStateOpen({}).value
          )) {
            e.classList.add(presentation.classMenuStateOpen({}).value);
          }
        }
        break;
      case "#view_modal_share":
        {
          buildRoot([
            stagingPageView,
            presentation.contModalViewShare({
              qr: /** @type {HTMLElement} */ (
                new DOMParser().parseFromString(
                  `
            <svg version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 580 580">
    <path fill="currentColor" d="M240 80h20v20h-20zm20 0h20v20h-20zm0 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm0 20h20v20h-20zm-80 20h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm-80 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm-80 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm-60 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm-80 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zM80 240h20v20H80zm80 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm-380 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm100 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm-360 20h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zM80 300h20v20H80zm20 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zM80 320h20v20H80zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-240 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-240 20h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-200 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm100 0h20v20h-20zm-240 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-200 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm100 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm-220 20h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm-180 20h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-240 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm40 0h20v20h-20z" shape-rendering="crispEdges"/>
    <path fill="currentColor" fill-rule="evenodd" d="M80 80h140v140H80Zm20 20h100v100H100Zm260-20h140v140H360Zm20 20h100v100H380Z" shape-rendering="crispEdges"/>
    <path fill="currentColor" d="M120 120h60v60h-60zm280 0h60v60h-60z" shape-rendering="crispEdges"/>
    <path fill="currentColor" fill-rule="evenodd" d="M80 360h140v140H80Zm20 20h100v100H100Z" shape-rendering="crispEdges"/>
    <path fill="currentColor" d="M120 400h60v60h-60z" shape-rendering="crispEdges"/>
  </svg>
            `,
                  "text/html"
                ).body.firstElementChild
              ),
              link: "https://a.b.c",
            }).root,
          ]);
        }
        break;
      case "#view_modal_node":
        {
          buildRoot([
            stagingPageView,
            presentation.contModalNode({
              currentListId: "ABCDEF-ABCDEF-ABCDEF-ABCDEF",
              currentListName: "ABCD",
              currentListLink: "abcd",
              nodeLink: "abcd",
            }).root,
          ]);
        }
        break;
      case "#fullscreen":
        {
          const media = document.createElement("div");
          media.style.border = "1px solid blue";
          buildRoot([
            stagingPageView,
            presentation.contMediaFullscreen({
              media: media,
            }).root,
          ]);
        }
        break;
      case "#form":
        {
          const errInput = presentation.leafInputPairText({
            id: "item2",
            title: "Text",
            value: "WXYC",
          });
          errInput.input.classList.add(
            presentation.classStateInvalid({}).value
          );
          const modInput = presentation.leafInputPairText({
            id: "item1",
            title: "Title",
            value: "ABCD",
          });
          modInput.input.classList.add(
            presentation.classStateModified({}).value
          );
          buildRoot([
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "Music" }).root,
              mainBody: presentation.contPageForm({
                entries: [
                  presentation.leafErrBlock({
                    data: "This is an error of greatest magnitude",
                    inRoot: false,
                  }).root,
                  presentation.leafInputPairText({
                    id: "item1",
                    title: "Title",
                    value: "ABCD",
                  }).root,
                  errInput.root,
                  modInput.root,
                  presentation.leafInputPairNumber({
                    id: "item2",
                    title: "Text",
                    value: "44",
                  }).root,
                  presentation.leafFormComment({
                    text: "This next item is a checkbox.\n\nThis text has multiple paragraphs.",
                  }).root,
                  presentation.leafInputPairBool({
                    id: "item2",
                    title: "Text",
                    value: true,
                  }).root,
                  presentation.leafInputPairDate({
                    id: "item2",
                    title: "Text",
                    value: "2024-08-23",
                  }).root,
                  presentation.leafInputPairTime({
                    id: "item2",
                    title: "Text",
                    value: "22:10:10",
                  }).root,
                  presentation.leafInputPairDatetime({
                    id: "item2",
                    title: "Text",
                    value: "2024-08-23T22:10:10",
                  }).root,
                  presentation.leafInputPairColor({
                    id: "item2",
                    title: "Text",
                    value: "#445566",
                  }).root,
                  presentation.leafInputPairFile({
                    id: "item3",
                    title: "Image",
                  }).root,
                  presentation.leafSpace({}).root,
                ],
                barChildren: [presentation.leafButtonBigCommit({}).root],
              }).root,
              menuBody: stagingMenu,
            }).root,
          ]);
        }
        break;
      case "#node_view":
        {
          const buildToolbar =
            /** @type { (download:boolean, center: boolean)=>HTMLElement} */ (
              download,
              center
            ) => {
              return presentation.contNodeToolbar({
                left: [],
                right: [
                  presentation.leafNodeViewToolbarHistoryLinkButton({
                    link: "https://abcd",
                  }).root,
                  ...(download
                    ? [
                        presentation.leafNodeViewToolbarDownloadLinkButton({
                          link: "https://abcd",
                        }).root,
                      ]
                    : []),
                  ...(center
                    ? [
                        presentation.leafNodeViewToolbarEditLinkButton({
                          link: "https://abcd",
                        }).root,
                        presentation.leafNodeViewToolbarEditListLinkButton({
                          link: "https://abcd",
                        }).root,
                      ]
                    : []),
                  presentation.leafNodeViewToolbarNodeButton({
                    link: "https://abcd",
                  }).root,
                ],
              }).root;
            };
          buildRoot([
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "Music" }).root,
              mainBody: presentation.contPageNodeView({
                children: [
                  presentation.contPageNodeSectionRel({
                    children: [
                      presentation.contNodeRowIncoming({
                        children: [
                          presentation.leafNodeViewNodeText({
                            value: "ABCD-1234",
                          }).root,
                          presentation.leafNodeViewPredicate({
                            value: "sunwet/1/is",
                          }).root,
                          buildToolbar(false, false),
                        ],
                        new: false,
                      }).root,
                      presentation.contNodeRowIncoming({
                        children: [
                          presentation.leafNodeViewNodeText({
                            value: "ABCD-1234",
                          }).root,
                          presentation.leafNodeViewPredicate({
                            value: "sunwet/1/has",
                          }).root,
                          presentation.leafMediaImg({ src: "testcover.jpg" })
                            .root,
                          buildToolbar(true, false),
                        ],
                        new: false,
                      }).root,
                    ],
                  }).root,
                  presentation.contNodeSectionCenter({
                    children: [
                      presentation.leafNodeViewNodeText({
                        value: "ABCD-1234",
                      }).root,
                      buildToolbar(false, true),
                    ],
                  }).root,
                  presentation.contPageNodeSectionRel({
                    children: [
                      presentation.contNodeRowOutgoing({
                        children: [
                          presentation.leafNodeViewPredicate({
                            value: "sunwet/1/has",
                          }).root,
                          presentation.leafNodeViewNodeText({
                            value: "ABCD-1234",
                          }).root,
                          buildToolbar(false, false),
                        ],
                        new: false,
                      }).root,
                      presentation.contNodeRowOutgoing({
                        children: [
                          presentation.leafNodeViewPredicate({
                            value: "sunwet/1/has",
                          }).root,
                          presentation.leafNodeViewNodeText({
                            value: "ABCD-1234",
                          }).root,
                          buildToolbar(false, false),
                        ],
                        new: false,
                      }).root,
                    ],
                  }).root,
                ],
              }).root,
              menuBody: stagingMenu,
            }).root,
          ]);
        }
        break;
      case "#node_edit":
        {
          /** @type { (args: {hint: string, value: string})=> Element} */
          buildRoot([
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "Music" }).root,
              mainBody: presentation.contPageNodeEdit({
                children: nodeEditChildren(1),
                barChildren: [
                  presentation.leafButtonBigDelete({}).root,
                  presentation.leafButtonBigCommit({}).root,
                ],
              }).root,
              menuBody: stagingMenu,
            }).root,
          ]);
        }
        break;
      case "#history":
        {
          buildRoot([
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "History" }).root,
              mainBody: presentation.contPageHistory({
                barChildren: [presentation.leafButtonBigCommit({}).root],
                children: [
                  presentation.contHistoryCommit({
                    stamp: new Date().toISOString(),
                    desc: "",
                  }).root,
                  presentation.contHistorySubject({
                    center: [
                      presentation.leafNodeViewNodeText({
                        value: "ABCD-1234",
                        link: "abcd",
                      }).root,
                    ],
                  }).root,
                  presentation.contHistoryPredicateObjectRemove({
                    children: [
                      presentation.leafNodeViewPredicate({
                        value: "sunwet/1/has",
                      }).root,
                      presentation.leafNodeViewNodeText({
                        value: "ABCD-1234",
                        link: "abcd",
                      }).root,
                    ],
                  }).root,
                  presentation.contHistoryPredicateObjectAdd({
                    children: [
                      presentation.leafNodeViewPredicate({
                        value: "sunwet/1/has",
                      }).root,
                      presentation.leafNodeViewNodeText({
                        value:
                          "ABCD-1234 this is a ton of data PLUS-A_VERY_LONG_IDENTIFIERleafNodeViewNodeText and some unbreakable",
                        link: "abcd",
                      }).root,
                    ],
                  }).root,
                  presentation.contHistorySubject({
                    center: [
                      presentation.leafNodeViewNodeText({
                        value: "ABCD-1234",
                        link: "abcd",
                      }).root,
                    ],
                  }).root,
                  presentation.contHistoryPredicateObjectRemove({
                    children: [
                      presentation.leafNodeViewPredicate({
                        value: "sunwet/1/has",
                      }).root,
                      presentation.leafNodeViewNodeText({
                        value: "ABCD-1234",
                        link: "abcd",
                      }).root,
                    ],
                  }).root,
                  presentation.contHistoryPredicateObjectAdd({
                    children: [
                      presentation.leafNodeViewPredicate({
                        value: "sunwet/1/has",
                      }).root,
                      presentation.leafNodeViewNodeText({
                        value: "ABCD-1234",
                        link: "abcd",
                      }).root,
                    ],
                  }).root,
                  presentation.contHistoryCommit({
                    stamp: new Date().toISOString(),
                    desc: "Something",
                  }).root,
                  presentation.contHistorySubject({
                    center: [
                      presentation.leafNodeViewNodeText({
                        value: "ABCD-1234",
                        link: "abcd",
                      }).root,
                    ],
                  }).root,
                  presentation.contHistoryPredicateObjectAdd({
                    children: [
                      presentation.leafNodeViewPredicate({
                        value: "sunwet/1/has",
                      }).root,
                      presentation.leafNodeViewNodeText({
                        value: "ABCD-1234",
                        link: "abcd",
                      }).root,
                    ],
                  }).root,
                  presentation.contHistoryPredicateObjectAdd({
                    children: [
                      presentation.leafNodeViewPredicate({
                        value: "sunwet/1/has",
                      }).root,
                      presentation.leafNodeViewNodeText({
                        value: "ABCD-1234",
                        link: "abcd",
                      }).root,
                    ],
                  }).root,
                  presentation.contHistorySubject({
                    center: [
                      presentation.leafNodeViewNodeText({
                        value: "ABCD-1234",
                        link: "abcd",
                      }).root,
                    ],
                  }).root,
                  presentation.contHistoryPredicateObjectRemove({
                    children: [
                      presentation.leafNodeViewPredicate({
                        value: "sunwet/1/has",
                      }).root,
                      presentation.leafNodeViewNodeText({
                        value: "ABCD-1234",
                        link: "abcd",
                      }).root,
                    ],
                  }).root,
                  presentation.contHistoryPredicateObjectRemove({
                    children: [
                      presentation.leafNodeViewPredicate({
                        value: "sunwet/1/has",
                      }).root,
                      presentation.leafNodeViewNodeText({
                        value: "ABCD-1234",
                        link: "abcd",
                      }).root,
                    ],
                  }).root,
                ],
              }).root,
              menuBody: stagingMenu,
            }).root,
          ]);
        }
        break;
      case "#query":
        {
          const jsonTab = presentation.contPageQueryTabJson({});
          jsonTab.jsonResults.textContent = JSON.stringify(
            [
              { key: "a", value: 4, link: "abcd" },
              { key: "banana", value: 6, link: "abcd" },
              { key: "c", value: -7, link: "abcd" },
            ],
            null,
            4
          );
          const downloadTab = presentation.contPageQueryTabDownloadKV({});
          const editTab = presentation.contPageQueryTabEdit({
            children: nodeEditChildren(10),
            barChildren: [
              presentation.leafButtonBigDelete({}).root,
              presentation.leafButtonBigCommit({}).root,
            ],
          });
          const root = presentation.contPageQuery({
            initialQuery: '"hello world" { => value }',
            downloadTab: [
              presentation.contStack({ children: [downloadTab.root] }).root,
            ],
            editTab: [
              presentation.contStack({
                children: [editTab.editBar, editTab.root],
              }).root,
            ],
            jsonTab: [
              presentation.contStack({ children: [jsonTab.root] }).root,
            ],
          });
          root.prettyResults.appendChild(
            presentation.contQueryPrettyRow({
              children: [
                presentation.leafQueryPrettyV({
                  value: "44444-444444-4444-4",
                  link: "abcd",
                }).root,
                presentation.leafQueryPrettyInlineKV({
                  key: "a",
                  value: "4",
                  link: "abcd",
                }).root,
                presentation.leafQueryPrettyInlineKV({
                  key: "banana",
                  value: "6",
                  link: "abcd",
                }).root,
                presentation.leafQueryPrettyInlineKV({
                  key: "c",
                  value: "-7",
                  link: "abcd",
                }).root,
              ],
            }).root
          );
          root.prettyResults.appendChild(
            presentation.contQueryPrettyRow({
              children: [
                presentation.leafQueryPrettyMediaV({
                  value: presentation.leafMediaImg({ src: "testcover.jpg" })
                    .root,
                  link: "abcd",
                }).root,
                presentation.leafQueryPrettyInlineKV({
                  key: "a",
                  value: "4",
                  link: "abcd",
                }).root,
                presentation.leafQueryPrettyMediaKV({
                  key: "noxos",
                  value: presentation.leafMediaImg({ src: "testcover.jpg" })
                    .root,
                  link: "abcd",
                }).root,
                presentation.leafQueryPrettyInlineKV({
                  key: "c",
                  value: "-7",
                  link: "abcd",
                }).root,
              ],
            }).root
          );
          root.prettyResults.appendChild(
            presentation.contQueryPrettyRow({
              children: [
                presentation.leafQueryPrettyInlineKV({
                  key: "a",
                  value: "4",
                  link: "abcd",
                }).root,
                presentation.leafQueryPrettyInlineKV({
                  key: "banana",
                  value: "6",
                  link: "abcd",
                }).root,
                presentation.leafQueryPrettyInlineKV({
                  key: "c",
                  value: "-7",
                  link: "abcd",
                }).root,
              ],
            }).root
          );
          downloadTab.downloadField.textContent = "file";
          downloadTab.downloadPattern.textContent = "{abc}-{def}";
          downloadTab.downloadResults.appendChild(
            presentation.leafQueryDownloadRow({
              link: "abcd",
              filename: "super_cityhall.txt",
            }).root
          );
          downloadTab.downloadResults.appendChild(
            presentation.leafErrBlock({
              data: "Bad data",
              inRoot: false,
            }).root
          );
          buildRoot([
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "Query" }).root,
              mainBody: root.root,
              menuBody: stagingMenu,
            }).root,
          ]);
        }
        break;
      case "#list_edit":
        {
          const root = presentation.contPageListEdit({
            backToViewLink: "abcd",
            children: [
              presentation.leafPageListEditEntry({
                id: "abcd-1234",
                idLink: "abcd",
                name: "Song abcd",
              }).root,
              presentation.leafPageListEditEntry({
                id: "efgh-5678",
                idLink: "abcd",
                name: "Song efgh",
              }).root,
            ],
          });
          buildRoot([
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "Edit list" }).root,
              mainBody: root.root,
              menuBody: stagingMenu,
            }).root,
          ]);
        }
        break;
      case "#link_perms":
        {
          document.body.appendChild(presentation.appLinkPerms({}).root);
        }
        break;
      case "#link_waiting":
        {
          document.body.appendChild(presentation.appLink({}).root);
        }
        break;
      case "#link":
        {
          const a = presentation.appLink({});
          const cover = document.createElement("img");
          cover.src = "testcover.jpg";
          a.displayOver.innerHTML = "";
          a.display.appendChild(cover);
          document.body.appendChild(a.root);
        }
        break;
      case "#media_comic":
        {
          const baseUrl =
            "/mnt/home-dev/r/server3/servers/main/stage/testmedia/xmen/";
          /** @type { {rtl: boolean, pages: {width: number, height: number, path: string }[]} } */
          const manifest = await (await fetch(`${baseUrl}sunwet.json`)).json();

          const children = [];
          var minAspect = 1;
          for (let i = 0; i < manifest.pages.length; i += 1) {
            const page = manifest.pages[i];

            const img = presentation.leafMediaComicPage({
              src: `${baseUrl}${page.path}`,
              aspectX: page.width.toString(),
              aspectY: page.height.toString(),
            }).root;
            const vertAspect = page.width / page.height;
            if (vertAspect < minAspect) {
              minAspect = vertAspect;
            }

            if (i == 0) {
              children.push(presentation.leafMediaComicEndPad({}).root);
            } else if (i % 2 == 1) {
              children.push(presentation.leafMediaComicMidPad({}).root);
            }
            children.push(img);
            if (i == manifest.pages.length - 1) {
              children.push(presentation.leafMediaComicEndPad({}).root);
            }
          }
          buildRoot([
            presentation.contMediaFullscreen({
              media: presentation.contMediaComicOuter({
                children: [
                  presentation.contMediaComicInner({
                    minAspectX: minAspect.toString(),
                    minAspectY: "1",
                    children: children,
                    rtl: true,
                  }).root,
                ],
              }).root,
            }).root,
          ]);
        }
        break;
      default:
        throw new Error();
    }
  });
}
