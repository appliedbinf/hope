#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

#[derive(Debug)]
pub struct HomopolymerRecord {
    pub contig: String,
    pub start: u32,
    pub stop: u32,
    pub base: String,
    pub length: u32,
}

impl HomopolymerRecord {
    pub fn print(&self) {
        println!("{:?}", self);
    }
}

#[derive(Debug, PartialEq)]
pub enum HomopolymerScore {
    Difference(i32),
    Other(String),
}

#[derive(Debug)]
pub struct HomopolymerResult<'a> {
    pub base: String, 
    pub homo_length: u32, 
    pub homo: HomopolymerRecord, 
    pub ra: &'a crate::read_alignment::ReadAlignment, 
    pub start: usize, 
    pub stop: usize, 
    pub region_read_aln: String,
    pub region_ref_aln: String,
    pub read_alignment: String, 
    pub ref_alignment: String, 
    pub read_upstream: String, 
    pub read_downstream: String, 
    pub ref_upstream: String, 
    pub ref_downstream: String, 
    pub length: u32, 
    pub score: HomopolymerScore, 
}

impl HomopolymerResult<'_> {
    pub fn new<'a>(homo: &'a HomopolymerRecord, ra: &'a crate::read_alignment::ReadAlignment, ref_seq: &'a String) -> HomopolymerResult<'a> {
        let start = ra.get_aligned_index(homo.start) as usize;
        let stop = ra.get_aligned_index(homo.stop) as usize;
        let up_idx = std::cmp::max(std::cmp::max(homo.start, 30) - 30, ra.pos as u32) as u32;
        let down_idx = (std::cmp::min(homo.stop, ra.aligned_end as u32 - 30) + 30) as u32;
        let upstart = ra.get_aligned_index(up_idx);
        let downstop = ra.get_aligned_index(down_idx);
        let (read_aln, ref_aln) = ra.extract_alignment(upstart, downstop, ref_seq);
        let aln_homo_start = start - upstart as usize;
        let aln_homo_stop = read_aln.len() - (downstop as usize - stop);
        
        let mut hr = HomopolymerResult {
            base: homo.base.to_string(),
            homo_length: homo.length,
            homo: HomopolymerRecord{
                contig: homo.contig.clone(),
                start: homo.start,
                stop: homo.stop,
                base: homo.base.clone(),
                length: homo.length,
            },
            ra: &ra,
            start: start,
            stop: stop,
            region_read_aln: read_aln.to_string(),
            region_ref_aln: ref_aln.to_string(),
            read_alignment: read_aln[aln_homo_start..aln_homo_stop].to_string(),
            ref_alignment: ref_aln[aln_homo_start..aln_homo_stop].to_string(),
            read_upstream: read_aln[..aln_homo_start].to_string(),
            read_downstream: read_aln[aln_homo_stop..].to_string(),
            ref_upstream: ref_aln[..aln_homo_start].to_string(),
            ref_downstream: ref_aln[aln_homo_stop..].to_string(),
            length: (stop-start) as u32,
            score: HomopolymerScore::Difference(0), 
        };
        hr.score();
        hr
    }

    pub fn score(&mut self) {
        let base = self.base.chars().nth(0).unwrap();

        // first check if we have flanking sequence to check
        if self.start == 0 || self.stop == self.region_read_aln.len() {
            self.score = HomopolymerScore::Other("skip".to_string());
            return
        }
        // next check for identical homopolymer with no flanking gaps
        // self.read_alignment.chars().all(|x| x != "-") possible alternative 
        if !self.ref_alignment.contains("-") && !self.read_alignment.contains("-") && !self.ref_upstream.ends_with(&[base, '-']) && !self.ref_downstream.starts_with(&[base, '-']) {
            if !self.read_alignment.chars().all(|x| x != '-') {
                self.score = HomopolymerScore::Other("mismatch".to_string());
                return
            } else {
                self.score = HomopolymerScore::Difference(0);
                return
            }
        }

        // Next handle identical homopolymer with flanking gaps in read
        if self.read_alignment.chars().all(|x| x != '-') && (self.read_upstream.ends_with('-') || self.read_downstream.starts_with('-')) {
            if self.read_upstream.ends_with('-') {
                let mut i = 1;
                let mut s = self.read_upstream.chars().nth(self.read_upstream.len()-i).unwrap();
                while s == '-' {
                    // homopolymer base in ref during deletion in read. uncertain what it means. return ?
                    if self.ref_upstream.chars().nth(self.read_upstream.len()-i).unwrap() == base || self.read_upstream.len() == 1 {
                        self.score = HomopolymerScore::Other("?".to_string());
                        return
                    }
                    i += 1;
                    s = self.read_upstream.chars().nth(self.read_upstream.len()-i).unwrap();
                    if i == self.read_upstream.len() {
                        self.score = HomopolymerScore::Other("?".to_string());
                        return
                    }
                }
                if self.read_upstream.chars().nth(self.read_upstream.len()-i).unwrap() == base {
                    // indel flanked by homopolymer base. Call it homopolymer-associated error
                    self.score = HomopolymerScore::Other("?".to_string());
                    return
                }
            }
            if self.read_downstream.starts_with("-") {
                let mut i = 0;
                let mut s = self.read_downstream.chars().nth(i).unwrap();
                while s == '-' {
                    if self.ref_downstream.chars().nth(i).unwrap() == base || self.read_downstream.len() == 1 {
                        self.score = HomopolymerScore::Other("?".to_string());
                        return
                    }
                    i += 1;
                    s = self.read_downstream.chars().nth(i).unwrap();
                    if i == self.read_downstream.len() {
                        self.score = HomopolymerScore::Other("?".to_string());
                        return
                    }
                }
                if self.read_downstream.chars().nth(i).unwrap() == base {
                    // indel flanked by homopolymer base. Call it homopolymer-associated error
                    self.score = HomopolymerScore::Other("?".to_string());
                    return
                }        
            }
            self.score = HomopolymerScore::Difference(0);
            return
        }
        
        // next handle extension of homopolymer in read
        if self.ref_alignment.contains("-") {
            let mut non_base = 0;
            for b in self.read_alignment.chars() {
                if b != base {
                    non_base += 1
                }
            }
            if non_base > self.read_alignment.len()/2 {
                self.score = HomopolymerScore::Other("?".to_string());
                return
            } else {
                self.score = HomopolymerScore::Difference(self.length as i32 - self.homo_length as i32);
                return
            }
        }
        // next handle deletions of homopolymer in read
        if self.read_alignment.contains("-") {
            // if any bases not the homopolymer base or gap, return "?"
            for b in self.read_alignment.chars() {
                if !vec![base, '-'].iter().any(|&i| i==b) {
                    self.score = HomopolymerScore::Other("?".to_string());
                    return
                }
            }
            // If not flanked by gaps in read, simply truncated homopolymer
            if !self.read_upstream.ends_with('-') && !self.read_downstream.starts_with('-') {
                // Check if majority of non-gap sequence not homopolymer base
                let mut non_base = 0;
                let mut is_base = 0;
                let mut gap = 0;
                for b in self.read_alignment.chars() {
                    if !vec![base, '-'].iter().any(|&i| i==b) {
                        non_base += 1
                    } else {
                        if b != base {
                            gap += 1
                        } else {
                            is_base += 1
                        }
                    }
                }
                if non_base > self.read_alignment.len()/2 {
                    self.score = HomopolymerScore::Other("?".to_string());
                    return
                } else {
                    self.score = HomopolymerScore::Difference(-gap as i32 - non_base as i32);
                    return
                }
            }
            // else, flanking deletion includes non-homopolymer base, return ?
            self.score = HomopolymerScore::Other("?".to_string());
            return
        }
        
        // Next handle insertions in read next to homopolymer
        if self.ref_upstream.ends_with('-') || self.ref_downstream.starts_with('-') {
            // Check if any inserted bases in the read are the homopolymer base. If so, return ?
            if self.ref_upstream.ends_with('-') {
                let mut i = 1;
                let mut s = self.ref_upstream.chars().nth(self.ref_upstream.len()-i).unwrap();
                while s == '-' {
                    if self.read_upstream.chars().nth(self.read_upstream.len()-i).unwrap() == base {
                        self.score = HomopolymerScore::Other("?".to_string());
                        return
                    }
                    i += 1;
                    s = self.ref_upstream.chars().nth(self.ref_upstream.len()-i).unwrap();
                    if i == self.ref_upstream.len()-1 {
                        self.score = HomopolymerScore::Other("?".to_string());
                        return
                    }
                }
                if self.read_upstream.chars().nth(self.read_upstream.len()-i).unwrap() == base {
                    // indel flanked by homopolymer base. Call it homopolymer-associated error
                    self.score = HomopolymerScore::Other("?".to_string());
                    return
                } 
            }
            if self.ref_downstream.starts_with('-') {
                let mut i = 0;
                let mut s = self.ref_downstream.chars().nth(i).unwrap();
                while s == '-' {
                    if self.read_downstream.chars().nth(i).unwrap() == base {
                        self.score = HomopolymerScore::Other("?".to_string());
                        return
                    }
                    i += 1;
                    s = self.ref_downstream.chars().nth(i).unwrap();
                    if i == self.ref_downstream.len()-1 {
                        self.score = HomopolymerScore::Other("?".to_string());
                        return
                    }
                }
                if self.read_downstream.chars().nth(i).unwrap() == base {
                    // indel flanked by homopolymer base. Call it homopolymer-associated error
                    self.score = HomopolymerScore::Other("?".to_string());
                    return
                } 
            }
            self.score = HomopolymerScore::Difference(0);
            return
        }
    }
}