// WIP: Chainable function transformations with a builder pattern API.
// Potentially useful for reusing image processing code

struct Trigger<Root, Current> {
  func: Box<dyn Fn(Root) -> Current>,
}

struct Image;

impl<Root> Trigger<Root, Root> {
  fn new() -> Trigger<Root, Root> {
    Trigger {
      func: Box::new(|x| x),
    }
  }
}

impl<Root: 'static, Current: 'static> Trigger<Root, Current> {
  fn map<X>(self, f: impl Fn(Current) -> X + 'static) -> Trigger<Root, X> {
    Trigger {
      func: Box::new(move |x| f((self.func)(x))),
    }
  }

  fn run(&self, root: Root) -> Current {
    (self.func)(root)
  }
}

impl<Root> Trigger<Root, Image> {
  fn crop(self, _x: u32, _y: u32, _w: u32, _h: u32) -> Trigger<Root, Image> {
    self
  }

  fn threshold(self, _r: u8, _g: u8, _b: u8, _threshold: u8) -> Trigger<Root, Image> {
    self
  }

  fn ocr(self) -> Trigger<Root, String> {
    Trigger {
      func: Box::new(|_| "".to_string()),
    }
  }
}

impl<Root> Trigger<Root, String> {
  fn regex(self, _pattern: &str) -> Trigger<Root, bool> {
    Trigger {
      func: Box::new(|_| true),
    }
  }
}

fn image_trigger() -> Trigger<Image, Image> {
  Trigger::new()
}

fn main() {
  image_trigger() // identity function on Image
    .crop(0, 0, 100, 100) // identity, then crop
    .threshold(255, 255, 255, 42) // identity => crop => threshold
    .ocr() // ...
    .regex("asdf")
    .map(|_| println!("Triggered!"))
    .run(Image);

  let my_string = Trigger::<String, String>::new()
    .map(|x| format!("({x})"))
    .map(|x| format!("[{x}]"))
    .map(|x| format!("{x}!"))
    .run("asdf".to_string());
  println!("{my_string}");
  assert_eq!(my_string, "[(asdf)]!");

  let _trigger = Trigger::<String, String>::new();
  image_trigger().run(Image);
}
