/* eslint-disable no-eq-null, eqeqeq */
function id(x) {
  return x;
}

export {id as fromBoolean};
export {id as fromNumber};
export {id as fromString};
export {id as fromArray};
export {id as fromObject};
export const jsonNull = null;

export function stringify(j) {
  return JSON.stringify(j);
}

export function stringifyWithIndent(i) {
  return function (j) {
    return JSON.stringify(j, null, i);
  };
}

function isArray(a) {
  return Object.prototype.toString.call(a) === "[object Array]";
}

export function _compare(EQ, GT, LT, a, b) {
  if (a == null) {
    if (b == null) return EQ;
    else return LT;
  } else if (typeof a === "boolean") {
    if (typeof b === "boolean") {
      // boolean / boolean
      if (a === b) return EQ;
      else if (a === false) return LT;
      else return GT;
    } else if (b == null) return GT;
    else return LT;
  } else if (typeof a === "number") {
    if (typeof b === "number") {
      if (a === b) return EQ;
      else if (a < b) return LT;
      else return GT;
    } else if (b == null) return GT;
    else if (typeof b === "boolean") return GT;
    else return LT;
  } else if (typeof a === "string") {
    if (typeof b === "string") {
      if (a === b) return EQ;
      else if (a < b) return LT;
      else return GT;
    } else if (b == null) return GT;
    else if (typeof b === "boolean") return GT;
    else if (typeof b === "number") return GT;
    else return LT;
  } else if (isArray(a)) {
    if (isArray(b)) {
      for (var i = 0; i < Math.min(a.length, b.length); i++) {
        var ca = _compare(EQ, GT, LT, a[i], b[i]);
        if (ca !== EQ) return ca;
      }
      if (a.length === b.length) return EQ;
      else if (a.length < b.length) return LT;
      else return GT;
    } else if (b == null) return GT;
    else if (typeof b === "boolean") return GT;
    else if (typeof b === "number") return GT;
    else if (typeof b === "string") return GT;
    else return LT;
  } else {
    if (b == null) return GT;
    else if (typeof b === "boolean") return GT;
    else if (typeof b === "number") return GT;
    else if (typeof b === "string") return GT;
    else if (isArray(b)) return GT;
    else {
      var akeys = Object.keys(a);
      var bkeys = Object.keys(b);
      if (akeys.length < bkeys.length) return LT;
      else if (akeys.length > bkeys.length) return GT;
      var keys = akeys.concat(bkeys).sort();
      for (var j = 0; j < keys.length; j++) {
        var k = keys[j];
        if (a[k] === undefined) return LT;
        else if (b[k] === undefined) return GT;
        var ck = _compare(EQ, GT, LT, a[k], b[k]);
        if (ck !== EQ) return ck;
      }
      return EQ;
    }
  }
}
export const caseJsonImpl = function (onNull) {
  return function (onBool) {
    return function (onNum) {
      return function (onStr) {
        return function (onArr) {
          return function (onObj) {
            return function (j) {
              if (j === null) return onNull();
              if (typeof j === "boolean") return onBool(j);
              if (typeof j === "number") return onNum(j);
              if (typeof j === "string") return onStr(j);
              if (Array.isArray(j)) return onArr(j);
              return onObj(j);
            };
          };
        };
      };
    };
  };
};

// Dedicated single-type case analysis, replacing the generic path that builds
// six constant callbacks per call. Dispatch matches caseJsonImpl.
export const _caseJsonNull = function (d) {
  return function (f) {
    return function (j) {
      return j == null ? f() : d;
    };
  };
};

export const _caseJsonBoolean = function (d) {
  return function (f) {
    return function (j) {
      return typeof j === "boolean" ? f(j) : d;
    };
  };
};

export const _caseJsonNumber = function (d) {
  return function (f) {
    return function (j) {
      return typeof j === "number" ? f(j) : d;
    };
  };
};

export const _caseJsonString = function (d) {
  return function (f) {
    return function (j) {
      return typeof j === "string" ? f(j) : d;
    };
  };
};

export const _caseJsonArray = function (d) {
  return function (f) {
    return function (j) {
      return Array.isArray(j) ? f(j) : d;
    };
  };
};

export const _caseJsonObject = function (d) {
  return function (f) {
    return function (j) {
      if (j == null || typeof j === "boolean" || typeof j === "number" || typeof j === "string" || Array.isArray(j)) return d;
      return f(j);
    };
  };
};
