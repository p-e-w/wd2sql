// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2022  Philipp Emanuel Weidmann <pew@worldwidemann.com>

use wikidata::{Fid, Lid, Pid, Qid, Sid};

pub fn q_id(id: Qid) -> u64 {
    id.0
}

pub fn p_id(id: Pid) -> u64 {
    id.0 + 1_000_000_000
}

pub fn l_id(id: Lid) -> u64 {
    id.0 + 2_000_000_000
}

pub fn f_id(id: Fid) -> u64 {
    l_id(id.0) + (id.1 as u64 * 100_000_000_000)
}

pub fn s_id(id: Sid) -> u64 {
    l_id(id.0) + (id.1 as u64 * 100_000_000_000) + 10_000_000_000
}
