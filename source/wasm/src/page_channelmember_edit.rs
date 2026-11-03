use {
    crate::{
        api::req_post_json,
        async_::bg_val,
        localdata::{
            self,
            LocalChannelMember,
            LocalContact,
            NowOrLater,
            get_or_req_api_channelmember,
            get_or_req_api_contact,
            req_api_channelmembers,
        },
        pageutil::{
            build_nol_form,
            req_form_portrait,
            FormPortrait,
        },
        state::{
            Ministate,
            MinistateChannelMember,
            goto_replace_ministate,
            state,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::Form,
    shared::interface::wire::c2s::{
        self,
    },
    std::rc::Rc,
};

#[derive(rooting_forms::Form)]
struct Form_ {
    #[title("Name")]
    name: String,
    #[title("Description")]
    description: String,
    #[title("Portrait")]
    portrait: FormPortrait,
    #[title("Can post")]
    can_post: bool,
}

async fn resolve<T: 'static + Clone>(v: NowOrLater<T>) -> Result<Option<T>, String> {
    match v {
        NowOrLater::Now(v) => return Ok(Some(v)),
        NowOrLater::Later(v) => return v.get().await,
    }
}

pub fn build(pc: &mut ProcessingContext, s: &MinistateChannelMember) -> El {
    let contact = get_or_req_api_contact(&s.identity, true);
    let member = get_or_req_api_channelmember(&s.channel, &s.identity);
    let both: NowOrLater<(LocalContact, LocalChannelMember)> = match (contact, member) {
        (NowOrLater::Now(c), NowOrLater::Now(m)) => NowOrLater::Now((c, m)),
        (contact, member) => NowOrLater::Later(bg_val(async move {
            let Some(c) = resolve(contact).await? else {
                return Ok(None);
            };
            let Some(m) = resolve(member).await? else {
                return Ok(None);
            };
            return Ok(Some((c, m)));
        })),
    };
    return build_nol_form(
        //. .
        pc,
        &Ministate::ChannelMember(MinistateChannelMember {
            channel: s.channel.clone(),
            identity: s.identity.clone(),
        }),
        "Edit member",
        both.map_async({
            let eg = pc.eg();
            let channel = s.channel.clone();
            async move |(contact, member)| {
                let portrait = req_form_portrait(&contact.res.portrait).await?;
                let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
                    name: contact.res.name.clone(),
                    description: contact.res.description.clone(),
                    portrait: portrait,
                    can_post: member.res.can_post,
                }));
                let form_state = Rc::new(form_state);
                return Ok((form_els.error, form_els.elements, async move |_idem| {
                    let Ok(new_values) = form_state.parse() else {
                        return Ok(());
                    };
                    let res = req_post_json(&state().env.base_url, c2s::ContactModify {
                        id: contact.res.id.clone(),
                        name: if new_values.name == contact.res.name {
                            None
                        } else {
                            Some(new_values.name)
                        },
                        description: if new_values.description == contact.res.description {
                            None
                        } else {
                            Some(new_values.description)
                        },
                        portrait: new_values.portrait.into_modify().await?,
                    }).await?;
                    localdata::ensure_contact(res.clone()).await;
                    if new_values.can_post != member.res.can_post {
                        req_post_json(&state().env.base_url, c2s::ChannelMemberModify {
                            channel: channel.clone(),
                            member: member.res.identity.clone(),
                            can_post: Some(new_values.can_post),
                        }).await?;
                        req_api_channelmembers(&channel, None).await?;
                    }
                    eg.event(|pc| {
                        goto_replace_ministate(pc, &state().log, &Ministate::ChannelMember(MinistateChannelMember {
                            channel: channel.clone(),
                            identity: res.id,
                        }));
                    }).unwrap();
                    return Ok(());
                }));
            }
        }),
    );
}
