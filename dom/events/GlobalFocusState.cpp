/* -*- Mode: C++; tab-width: 8; indent-tabs-mode: nil; c-basic-offset: 2 -*- */
/* vim: set ts=8 sts=2 et sw=2 tw=80: */
/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#include "mozilla/GlobalFocusState.h"

using namespace mozilla;

mozilla::layers::LayersId GlobalFocusState::sFocusedLayersId{0};

mozilla::dom::TabParent* GlobalFocusState::FocusedTabParent() {
  return mozilla::dom::TabParent::GetTabParentFromLayersId(sFocusedLayersId);
}

void GlobalFocusState::SetFocusedLayersId(
    const mozilla::layers::LayersId& aLayersId) {
  sFocusedLayersId = aLayersId;
}
