#version 450

layout(location = 0) in flat uint identifier;

layout(location = 0) out uint fragment_color;

void main() {
    fragment_color = identifier;
}
