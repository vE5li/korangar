#version 450

layout(location = 0) in vec2 position;

layout(location = 0) out vec4 fragment_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput interface_in;

void main() {
    fragment_color = subpassLoad(interface_in);
}
