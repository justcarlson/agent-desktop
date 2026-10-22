use super::ReadPurpose;

#[test]
fn shape_reads_the_child_list_and_none_of_the_evidence() {
    let purpose = ReadPurpose::Shape;
    assert!(purpose.children());
    assert!(!purpose.identity());
    assert!(!purpose.vocabulary());
    assert!(!purpose.value());
    assert!(!purpose.bounds());
    assert!(!purpose.actions());
}
