# 0066 Examples whose printed answer is a comment

Status: closed (not a defect)
Milestone: v1
Area: tests

Two examples on the units notes print a remark where an answer would be: `10m × 10m | 100 m² // area, a supported unit type` and `3 kg × 3 liters | // Not "9 kg liters", this unit does not exist`. The first agrees with the engine once the remark is removed. The second says the reference app treats `kg × liters` as an unsupported unit combination; the engine prints `9 L·kg`. Listed as known; whether the reference app refuses the product or prints it differently is a question for a real install.
