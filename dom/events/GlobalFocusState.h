/* -*- Mode: C++; tab-width: 8; indent-tabs-mode: nil; c-basic-offset: 2 -*- */
/* vim: set ts=8 sts=2 et sw=2 tw=80: */
/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#ifndef mozilla_GlobalFocusState_h
#define mozilla_GlobalFocusState_h

#include "mozilla/layers/LayersTypes.h"
#include "mozilla/dom/TabParent.h"

namespace mozilla {

class GlobalFocusState {
 public:
  static mozilla::dom::TabParent* FocusedTabParent();
  static void SetFocusedLayersId(const mozilla::layers::LayersId& aLayersId);

 private:
  /**
   * Chrome-process cached copy of APZ's notion of currently-focused
   * process. Set by APZCCallbackHelper::NotifyFocusLayersIdChanged.
   * Read only from the main thread of the chrome process.
   */
  static mozilla::layers::LayersId sFocusedLayersId;
};

};  // namespace mozilla

#endif  // mozilla_GlobalFocusState_h
