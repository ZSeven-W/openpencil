//! One complete output shape shared by every planning prompt mode.

pub(crate) const OUTPUT_SHAPE: &str = r##"
PLAN OUTPUT SHAPE (adapt dimensions and sections to the brief):
{"rootFrame":{"id":"page","name":"Page","width":375,"height":812,"layout":"vertical","gap":16,"fill":[{"type":"solid","color":"#FAFAFA"}]},"styleGuideName":"guide-name","sharedContext":{"sourceFacts":[],"exampleEntities":[{"id":"subject-1","name":"Example subject","imageSubject":"same subject for every image","attributes":{"field":"value"},"usedBy":["section-id"]}]},"subtasks":[{"id":"section-id","label":"Section","elements":"owned UI elements","region":{"width":375,"height":240}}]}
rootFrame, sharedContext and subtasks are SIBLINGS. Always include sharedContext; use {} when nothing is shared. Each usedBy entry must exactly equal an id in final subtasks, not a label or draft id. Attribute values are strings, including prices/counts.
"##;
