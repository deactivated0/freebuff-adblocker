#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureState {
    Unpatched,
    AlreadyPatched,
    Missing,
    Ambiguous,
}

#[derive(Debug, Clone, Copy)]
pub struct Patch {
    pub id: usize,
    pub name: &'static str,
    pub surface: &'static str,
    pub description: &'static str,
    pub original: &'static [u8],
    pub replacement: &'static [u8],
}

pub const ALL_PATCHES: [Patch; 13] = [
    // Patch 01: uJ (Freebuff adsEnabled bypass)
    Patch {
        id: 1,
        name: "uJ_global_ads_enabled",
        surface: "Ad Settings",
        description: "Bypass global adsEnabled logic in Freebuff mode (force false)",
        original: b"uJ=()=>{if(CA)return!0;",
        replacement: b"uJ=()=>{if(CA)return!1;",
    },
    // Patch 02: yyA (Disable ad refusal in free mode)
    Patch {
        id: 2,
        name: "yyA_allow_ads_disable",
        surface: "Ad Settings",
        description: "Bypass refusal to disable ads in Freebuff mode",
        original: b"yyA=(H={})=>{if(H.isFreebuff??CA)return jH.info(",
        replacement: b"yyA=(H={})=>{if(!1/*__________*/)return jH.info(",
    },
    // Patch 03: yRA (Default settings adsEnabled: false)
    Patch {
        id: 3,
        name: "yRA_default_settings",
        surface: "Ad Settings",
        description: "Set default settings initialization adsEnabled to false",
        original: b"yRA={mode:\"DEFAULT\",adsEnabled:!0}",
        replacement: b"yRA={mode:\"DEFAULT\",adsEnabled:!1}",
    },
    // Patch 04: hY (Central ad request network block)
    Patch {
        id: 4,
        name: "hY_ad_request_network",
        surface: "Ad Requests",
        description: "Central ad request builder returns null immediately (no network calls)",
        original: b"async function hY(H){let A=zL();if(!A)",
        replacement: b"async function hY(H){return null;/*_*/",
    },
    // Patch 05: iaL (Ad policy / dock arm request network block)
    Patch {
        id: 5,
        name: "iaL_ad_policy_fetch",
        surface: "Ad Requests",
        description: "Ad policy and dock arm fetch returns control without network calls",
        original: b"async function iaL(H=fetch,A=ZM){let $=zL();if(!$)return\"control\";",
        replacement: b"async function iaL(H=fetch,A=ZM){return\"control\";/*_____________*/",
    },
    // Patch 06: fzL (Dock ad component block)
    Patch {
        id: 6,
        name: "fzL_dock_ad_component",
        surface: "Dock Ads",
        description: "Dock ad UI component fzL returns null immediately",
        original: b"fzL=({ad:H,width:A,expanded:$=!1,chordHint:L,onToggle:_,onClick:f,onImpression:I})=>",
        replacement: b"fzL=()=>null,fz0=(/*___________________________________________________________*/)=>",
    },
    // Patch 07: dRL (Composer & inline response ads gate)
    Patch {
        id: 7,
        name: "dRL_composer_inline_gate",
        surface: "Composer & Inline Ads",
        description: "Force dRL ad gate true to suppress composer and inline response ads",
        original: b"function dRL(H){let A=H.terminalHeight<=17;return!H.enabled||A&&!H.isFreeMode}",
        replacement: b"function dRL(H){return!0;}/*________________________________________________*/",
    },
    // Patch 08: IcA (Splash ad component block)
    Patch {
        id: 8,
        name: "IcA_splash_ad_component",
        surface: "Splash Ads",
        description: "Splash ad UI component IcA returns null immediately",
        original: b"function IcA({width:H,variant:A,placementId:$=tX.cliSplash,enabled:L=st(\"cliSplash\"),getServed:_,open:f=Kf}){",
        replacement: b"function IcA(){return null}function Ic0(/*_______________________________________________________________*/){",
    },
    // Patch 09: obA (Statusline ad component block)
    Patch {
        id: 9,
        name: "obA_statusline_ad_component",
        surface: "Statusline Ads",
        description: "Statusline ad UI component obA returns null immediately",
        original: b"function obA({placementId:H=tX.cliStatusline,enabled:A=st(\"cliStatusline\"),getServed:$,open:L=Kf,dealsLive:_}){",
        replacement: b"function obA(){return null}function ob0(/*_________________________________________________________________*/){",
    },
    // Patch 10: vkH (Display ads active flag)
    Patch {
        id: 10,
        name: "vkH_display_ads_active",
        surface: "Statusline & Splash Ads",
        description: "Display ads active checker vkH returns false for all placements",
        original: b"function vkH(H,A){return q4$[H]||G4$(A).has(H)}",
        replacement: b"function vkH(H,A){return!1;}/*_______________*/",
    },
    // Patch 11: FA$ (Sponsored survey component block)
    Patch {
        id: 11,
        name: "FA$_sponsored_survey",
        surface: "Sponsored Surveys",
        description: "Sponsored survey builder FA$ returns null immediately",
        original: b"function FA$(H,A){if(H.questions.some((_)=>!wc.includes(_.kind)))return null;",
        replacement: b"function FA$(H,A){return null;/*___________________________________________*/",
    },
    // Patch 12: r0$ (Partner slash command menu ad injection block)
    Patch {
        id: 12,
        name: "r0$_partner_slash_ad",
        surface: "Partner Ads",
        description: "Suppress partner review ad injection in slash command suggestions menu",
        original: b"let T=H.findIndex((B)=>B.id===\"review\");if(!_||!I||T<0)return H;let v=[...H];",
        replacement: b"return H;let v=[...H];/*___________________________________________________*/",
    },
    // Patch 13: BJ (Sponsored task dock / proposal block)
    Patch {
        id: 13,
        name: "BJ_sponsored_proposal_dock",
        surface: "Sponsored Task Docks",
        description: "Sponsored proposal block identifier BJ returns false",
        original: b"function BJ(H){return H.type===\"sponsored-proposal\"}",
        replacement: b"function BJ(H){return!1;}/*_______________________*/",
    },
];

#[derive(Debug, Clone)]
pub struct PatchScanResult {
    pub patch: Patch,
    pub state: SignatureState,
    pub original_offset: Option<usize>,
    pub original_count: usize,
    pub replacement_offset: Option<usize>,
    pub replacement_count: usize,
}

pub fn count_and_first_offset(haystack: &[u8], needle: &[u8]) -> (usize, Option<usize>) {
    if needle.is_empty() || haystack.len() < needle.len() {
        return (0, None);
    }

    let first = needle[0];
    let needle_len = needle.len();
    let mut count = 0;
    let mut first_offset = None;
    let mut rest = haystack;
    let mut offset = 0;

    while rest.len() >= needle_len {
        let max_search = rest.len() - needle_len + 1;
        match rest[..max_search].iter().position(|&b| b == first) {
            Some(pos) => {
                let curr_offset = offset + pos;
                if &rest[pos..pos + needle_len] == needle {
                    if first_offset.is_none() {
                        first_offset = Some(curr_offset);
                    }
                    count += 1;
                    offset += pos + needle_len;
                    rest = &rest[pos + needle_len..];
                } else {
                    offset += pos + 1;
                    rest = &rest[pos + 1..];
                }
            }
            None => break,
        }
    }

    (count, first_offset)
}

pub fn scan_binary(data: &[u8]) -> Vec<PatchScanResult> {
    ALL_PATCHES
        .iter()
        .map(|patch| {
            let (orig_count, orig_off) = count_and_first_offset(data, patch.original);
            let (repl_count, repl_off) = count_and_first_offset(data, patch.replacement);

            let state = if orig_count == 1 && repl_count == 0 {
                SignatureState::Unpatched
            } else if orig_count == 0 && repl_count == 1 {
                SignatureState::AlreadyPatched
            } else if orig_count > 1 || repl_count > 1 {
                SignatureState::Ambiguous
            } else {
                SignatureState::Missing
            };

            PatchScanResult {
                patch: *patch,
                state,
                original_offset: orig_off,
                original_count: orig_count,
                replacement_offset: repl_off,
                replacement_count: repl_count,
            }
        })
        .collect()
}

pub fn apply_patches_in_memory(data: &mut [u8]) -> Result<usize, String> {
    let scan = scan_binary(data);
    let mut applied = 0;

    // Verify all signatures are in a valid state before touching the buffer
    for res in &scan {
        match res.state {
            SignatureState::Unpatched => {}
            SignatureState::AlreadyPatched => {}
            SignatureState::Ambiguous => {
                return Err(format!(
                    "Signature #{} ({}) is ambiguous: found {} originals, {} replacements",
                    res.patch.id, res.patch.name, res.original_count, res.replacement_count
                ));
            }
            SignatureState::Missing => {
                return Err(format!(
                    "Signature #{} ({}) was not found in the binary",
                    res.patch.id, res.patch.name
                ));
            }
        }
    }

    // Apply unpatched signatures
    for res in &scan {
        if res.state == SignatureState::Unpatched {
            let off = res.original_offset.unwrap();
            let orig_len = res.patch.original.len();
            let repl_len = res.patch.replacement.len();
            assert_eq!(
                orig_len, repl_len,
                "Length mismatch for patch #{}",
                res.patch.id
            );
            data[off..off + repl_len].copy_from_slice(res.patch.replacement);
            applied += 1;
        }
    }

    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_13_patches_lengths_match() {
        assert_eq!(ALL_PATCHES.len(), 13);
        for patch in &ALL_PATCHES {
            assert_eq!(
                patch.original.len(),
                patch.replacement.len(),
                "Patch #{} ({}) length mismatch: {} != {}",
                patch.id,
                patch.name,
                patch.original.len(),
                patch.replacement.len()
            );
        }
    }

    #[test]
    fn test_patching_synthetic_buffer() {
        let mut buf = Vec::new();
        for patch in &ALL_PATCHES {
            buf.extend_from_slice(b"prefix-");
            buf.extend_from_slice(patch.original);
            buf.extend_from_slice(b"-suffix\n");
        }

        let orig_len = buf.len();
        let scan_before = scan_binary(&buf);
        for res in &scan_before {
            assert_eq!(res.state, SignatureState::Unpatched);
        }

        let applied = apply_patches_in_memory(&mut buf).expect("apply must succeed");
        assert_eq!(applied, 13);
        assert_eq!(buf.len(), orig_len);

        let scan_after = scan_binary(&buf);
        for res in &scan_after {
            assert_eq!(res.state, SignatureState::AlreadyPatched);
        }

        // Re-applying is idempotent
        let re_applied = apply_patches_in_memory(&mut buf).expect("idempotent apply");
        assert_eq!(re_applied, 0);
    }
}
