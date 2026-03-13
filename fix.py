with open("src/core/summing_mixer.rs", "r") as f:
    lines = f.readlines()

new_lines = []
for line in lines:
    if "let mut output = String::new();" in line:
        new_lines.append("        use core::fmt::Write;\n")
        new_lines.append(line)
    else:
        new_lines.append(line)

with open("src/core/summing_mixer.rs", "w") as f:
    f.writelines(new_lines)
