# Construction office evidence

Shows one synthetic design source attached to an existing office design
register. The construction fact keeps the precise
`construction-evidence/design-source` relation, original external reference,
visibility, and accepted state. The office evidence row stores only a pointer
to that immutable fact and evidence slot, using the existing broad
`office/source-document` role.

Resolution requires the project-read and office evidence-read capabilities.
Attachment also requires office evidence-write. The bridge checks the exact
project and visibility before touching the office store. Repeating the attach
does not create a second row, and the office link proves provenance without
changing construction acceptance.
