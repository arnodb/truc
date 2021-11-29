use crate::{
    filter::{
        anchor::anchorize,
        fork::extract_fields::extract_fields,
        group::group,
        in_place::string::{reverse_chars_boxed_str, to_lowercase_boxed_str},
        sort::sort,
    },
    graph::{GraphBuilder, Node, NodeCluster},
    stream::NodeStream,
    support::FullyQualifiedName,
};

fn build_rev_table(
    graph: &mut GraphBuilder,
    name: FullyQualifiedName,
    inputs: [NodeStream; 1],
    token_field: &str,
    reference_field: &str,
) -> NodeCluster<1, 2> {
    let [input] = inputs;

    let extract_token = extract_fields(
        graph,
        name.sub("extract_token"),
        [input.clone()],
        &[token_field, reference_field],
    );
    let reverse_token = reverse_chars_boxed_str(
        graph,
        name.sub("reverse_token"),
        [extract_token.outputs()[1].clone()],
        [token_field].into_iter(),
    );
    let sort_token = sort(
        graph,
        name.sub("sort_token"),
        [reverse_token.outputs()[0].clone()],
        &[token_field],
    );

    let outputs = [
        extract_token.outputs()[0].clone(),
        sort_token.outputs()[0].clone(),
    ];
    NodeCluster::new(
        name,
        vec![
            Box::new(extract_token),
            Box::new(reverse_token),
            Box::new(sort_token),
        ],
        [input],
        outputs,
    )
}

fn build_sim_table(
    graph: &mut GraphBuilder,
    name: FullyQualifiedName,
    inputs: [NodeStream; 1],
    token_field: &str,
    reference_field: &str,
    ref_rs_field: &str,
) -> NodeCluster<1, 2> {
    let [input] = inputs;

    let extract_token = extract_fields(
        graph,
        name.sub("extract_token"),
        [input.clone()],
        &[token_field, reference_field],
    );

    let simplify_token = to_lowercase_boxed_str(
        graph,
        name.sub("simplify_token"),
        [extract_token.outputs()[1].clone()],
        [token_field].into_iter(),
    );
    let sort_token = sort(
        graph,
        name.sub("sort_token"),
        [simplify_token.outputs()[0].clone()],
        &[token_field, reference_field],
    );
    let group = group(
        graph,
        name.sub("group"),
        [sort_token.outputs()[0].clone()],
        &[reference_field],
        ref_rs_field,
    );

    let outputs = [
        extract_token.outputs()[0].clone(),
        group.outputs()[0].clone(),
    ];
    NodeCluster::new(
        name,
        vec![
            Box::new(extract_token),
            Box::new(simplify_token),
            Box::new(sort_token),
            Box::new(group),
        ],
        [input],
        outputs,
    )
}

pub fn build_word_list(
    graph: &mut GraphBuilder,
    name: FullyQualifiedName,
    inputs: [NodeStream; 1],
    token_field: &str,
    anchor_field: &str,
    sim_anchor_field: &str,
    sim_rs_field: &str,
) -> NodeCluster<1, 4> {
    let [input] = inputs;

    let sim = build_sim_table(
        graph,
        name.sub("sim"),
        [input.clone()],
        token_field,
        anchor_field,
        sim_rs_field,
    );
    let rev = build_rev_table(
        graph,
        name.sub("rev"),
        [sim.outputs()[0].clone()],
        token_field,
        anchor_field,
    );

    let anchorize = anchorize(
        graph,
        name.sub("anchorize"),
        [sim.outputs()[1].clone()],
        sim_anchor_field,
    );
    let sim_rev = build_rev_table(
        graph,
        name.sub("sim_rev"),
        [anchorize.outputs()[0].clone()],
        token_field,
        sim_anchor_field,
    );

    let outputs = [
        rev.outputs()[0].clone(),
        rev.outputs()[1].clone(),
        sim_rev.outputs()[0].clone(),
        sim_rev.outputs()[1].clone(),
    ];
    NodeCluster::new(
        name,
        vec![
            Box::new(sim),
            Box::new(rev),
            Box::new(anchorize),
            Box::new(sim_rev),
        ],
        [input],
        outputs,
    )
}
