// Side accents inside styled-components templates. The template's own
// declarations style its element, so a nested bar or state rule reads the
// template's radius. Each case uses a unique width so every finding
// attributes to one.
import styled from 'styled-components';

export const FlagTemplatePseudo = styled.div`
  position: relative;
  border-radius: 12px;
  &::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: #0f766e;
  }
`;

export const FlagTemplateState = styled.div`
  border-radius: 12px;
  &.active {
    border-left: 5px solid #0f766e;
  }
`;

export const PassTemplatePseudo = styled.div`
  position: relative;
  &::after {
    content: "";
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    width: 4px;
    background: #0f766e;
  }
`;

export const PassTemplateState = styled.div`
  padding: 16px;
  &.active {
    border-left: 6px solid #0f766e;
  }
`;
