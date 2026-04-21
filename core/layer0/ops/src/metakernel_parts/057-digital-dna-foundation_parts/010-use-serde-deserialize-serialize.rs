use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const DIGITAL_DNA_SCHEMA_VERSION: &str = "v1";
const GENE_START_MARKER: &str = "GENE_START";
const GENE_STOP_MARKER: &str = "GENE_STOP";
const DIGITAL_DNA_DEFAULT_PARENT_SIGNATURE: &str = "root";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Quark {
    value: i8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Baryon {
    q1: Quark,
    q2: Quark,
    q3: Quark,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Letter {
    core: Baryon,
    func: Baryon,
    #[serde(rename = "mod")]
    mod_: Baryon,
    verity: Baryon,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Codon {
    letters: [Letter; 4],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Gene {
    start_marker: String,
    stop_marker: String,
    repair_enabled: bool,
    codons: Vec<Codon>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct GenomeHeader {
    instance_id: String,
    parent_signature: String,
    version: String,
    birth_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct InstanceDna {
    header: GenomeHeader,
    genes: Vec<Gene>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct DigitalDnaState {
    schema_version: String,
    genomes: BTreeMap<String, InstanceDna>,
    last_receipt_hash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct QubitWrapper<T> {
    layer: String,
    superposed: Vec<T>,
    collapsed_index: Option<usize>,
    semantics: String,
}

impl Default for DigitalDnaState {
    fn default() -> Self {
        Self {
            schema_version: DIGITAL_DNA_SCHEMA_VERSION.to_string(),
            genomes: BTreeMap::new(),
            last_receipt_hash: None,
        }
    }
}

impl Quark {
    fn new(value: i8) -> Result<Self, String> {
        if matches!(value, -1 | 0 | 1) {
            Ok(Self { value })
        } else {
            Err("quark_value_out_of_range".to_string())
        }
    }

    fn complement(&self) -> Self {
        Self { value: -self.value }
    }
}

impl Baryon {
    fn from_trits(values: [i8; 3]) -> Result<Self, String> {
        Ok(Self {
            q1: Quark::new(values[0])?,
            q2: Quark::new(values[1])?,
            q3: Quark::new(values[2])?,
        })
    }

    fn values(&self) -> [i8; 3] {
        [self.q1.value, self.q2.value, self.q3.value]
    }

    fn complement(&self) -> Self {
        Self {
            q1: self.q1.complement(),
            q2: self.q2.complement(),
            q3: self.q3.complement(),
        }
    }
}

impl Letter {
    fn new(core: Baryon, func: Baryon, mod_: Baryon) -> Self {
        let verity = derive_verity(&core, &func, &mod_);
        Self {
            core,
            func,
            mod_,
            verity,
        }
    }

    fn is_valid(&self) -> bool {
        self.verity == derive_verity(&self.core, &self.func, &self.mod_)
    }
}

impl Codon {
    fn new(letters: [Letter; 4]) -> Result<Self, String> {
        if letters.iter().all(Letter::is_valid) {
            Ok(Self { letters })
        } else {
            Err("codon_contains_invalid_letter".to_string())
        }
    }

    fn is_valid(&self) -> bool {
        self.letters.iter().all(Letter::is_valid)
    }
}

fn trit_from_byte(byte: u8) -> i8 {
    match byte % 3 {
        0 => -1,
        1 => 0,
        _ => 1,
    }
}

fn zero_baryon() -> Baryon {
    Baryon::from_trits([0, 0, 0]).expect("zero trits valid")
}

fn zero_letter() -> Letter {
    Letter::new(zero_baryon(), zero_baryon(), zero_baryon())
}

// TODO-NORMATIVE: Standardize verity derivation across all runtime surfaces once Digital DNA v2 is approved.
fn derive_verity(core: &Baryon, func: &Baryon, mod_: &Baryon) -> Baryon {
    let mut hasher = Sha256::new();
    hasher.update(
        format!(
            "{:?}|{:?}|{:?}",
            core.values(),
            func.values(),
            mod_.values()
        )
        .as_bytes(),
    );
    let digest = hasher.finalize();
    Baryon::from_trits([
        trit_from_byte(digest[0]),
        trit_from_byte(digest[1]),
        trit_from_byte(digest[2]),
    ])
    .unwrap_or_else(|_| zero_baryon())
}

fn is_complement(a: &Baryon, b: &Baryon) -> bool {
    a.complement() == *b
}

fn seeded_baryon(seed: &str, scope: &str) -> Baryon {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    hasher.update(b"|");
    hasher.update(scope.as_bytes());
    let digest = hasher.finalize();
    Baryon::from_trits([
        trit_from_byte(digest[0]),
        trit_from_byte(digest[1]),
        trit_from_byte(digest[2]),
    ])
    .unwrap_or_else(|_| zero_baryon())
}

fn seeded_letter(seed: &str, scope: &str) -> Letter {
    let core = seeded_baryon(seed, &format!("{scope}:core"));
    let func = seeded_baryon(seed, &format!("{scope}:func"));
    let mod_ = seeded_baryon(seed, &format!("{scope}:mod"));
    Letter::new(core, func, mod_)
}

fn seeded_codon(seed: &str, index: usize) -> Codon {
    let letters = [
        seeded_letter(seed, &format!("codon_{index}_0")),
        seeded_letter(seed, &format!("codon_{index}_1")),
        seeded_letter(seed, &format!("codon_{index}_2")),
        seeded_letter(seed, &format!("codon_{index}_3")),
    ];
    Codon::new(letters).unwrap_or_else(|_| {
        Codon::new([zero_letter(), zero_letter(), zero_letter(), zero_letter()])
            .expect("fallback codon must remain valid")
    })
}

fn default_gene(seed: &str) -> Gene {
    Gene {
        start_marker: GENE_START_MARKER.to_string(),
        stop_marker: GENE_STOP_MARKER.to_string(),
        repair_enabled: true,
        codons: vec![seeded_codon(seed, 0)],
    }
}

fn validate_instance_dna(genome: &InstanceDna) -> Result<(), String> {
    if !is_token_id(&genome.header.instance_id) {
        return Err("instance_id_invalid".to_string());
    }
    if genome.header.version.trim().is_empty() {
        return Err("genome_header_version_required".to_string());
    }
    if genome.genes.is_empty() {
        return Err("genome_requires_at_least_one_gene".to_string());
    }
    for gene in &genome.genes {
        if gene.start_marker != GENE_START_MARKER {
            return Err("gene_start_marker_invalid".to_string());
        }
        if gene.stop_marker != GENE_STOP_MARKER {
            return Err("gene_stop_marker_invalid".to_string());
        }
        if gene.codons.is_empty() {
            return Err("gene_requires_at_least_one_codon".to_string());
        }
        if !gene.codons.iter().all(Codon::is_valid) {
            return Err("gene_contains_invalid_codon".to_string());
        }
    }
    Ok(())
}
