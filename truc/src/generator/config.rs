use super::fragment::FragmentGenerator;

#[derive(Default)]
pub struct GeneratorConfig {
    pub custom_fragment_generators: Vec<Box<dyn FragmentGenerator>>,
}
